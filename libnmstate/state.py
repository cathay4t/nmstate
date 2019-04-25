#
# Copyright 2019 Red Hat, Inc.
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 2 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#

try:
    from collections.abc import Mapping
except ImportError:
    from collections import Mapping

from collections import defaultdict
import copy
import logging
from operator import itemgetter
import six

from libnmstate import iplib
from libnmstate import metadata
from libnmstate.error import NmstateVerificationError
from libnmstate.prettystate import format_desired_current_state_diff
from libnmstate.schema import Interface
from libnmstate.schema import InterfaceType
from libnmstate.schema import InterfaceState
from libnmstate.schema import Route
from libnmstate.nm import route as nm_route


class _Route(object):
    def __init__(self, route):
        self.state = copy.deepcopy(route)
        if route.get(Route.STATE) != Route.ABSENT:
            if Route.TABLE_ID not in route:
                self.state[Route.TABLE_ID] = \
                    nm_route.NM_ROUTE_TABLE_USE_DEFAULT_CFG
            if Route.METRIC not in route:
                self.state[Route.METRIC] = nm_route.NM_ROUTE_DEFAULT_METRIC
            if Route.NEXT_HOP_ADDRESS not in route:
                self.state[Route.NEXT_HOP_ADDRESS] = ''

    def __hash__(self):
        return hash((self.state.get(Route.STATE),
                     self.state.get(Route.TABLE_ID),
                     self.state.get(Route.DESTINATION),
                     self.state.get(Route.NEXT_HOP_INTERFACE),
                     self.state.get(Route.NEXT_HOP_ADDRESS),
                     self.state.get(Route.METRIC)))

    def __eq__(self, other):
        return self.state == other.state

    def is_match(self, other):
        """
        Assuming the other state contains all the properties to compare.
        This is OK as other state is always current route state.
        """
        for prop_name in (Route.DESTINATION, Route.NEXT_HOP_INTERFACE,
                          Route.NEXT_HOP_ADDRESS, Route.METRIC,
                          Route.TABLE_ID):
            if prop_name in self.state and \
               self.state[prop_name] != other.state[prop_name]:
                return False
        return True


def create_state(state, interfaces_to_filter=None):
    """
    Create a state object, given an initial state.
    interface_filter: Limit the interfaces included in the state to the ones
    mentioned in the list. None implied no filtering.
    """
    new_state = {}
    if interfaces_to_filter is not None:
        origin = State(state)
        iface_names = set(origin.interfaces) & interfaces_to_filter
        filtered_ifaces_state = [
            origin.interfaces[ifname] for ifname in iface_names
        ]
        new_state[Interface.KEY] = filtered_ifaces_state

    return State(new_state)


class State(object):
    def __init__(self, state):
        self._state = copy.deepcopy(state)
        self._ifaces_state = State._index_interfaces_state_by_name(self._state)

    def __eq__(self, other):
        return self.state == other.state

    def __hash__(self):
        return hash(self.state)

    def __str__(self):
        return str(self.state)

    def __repr__(self):
        return self.__str__()

    @property
    def state(self):
        self._state[Interface.KEY] = sorted(
            list(six.viewvalues(self._ifaces_state)),
            key=itemgetter('name')
        )
        return self._state

    @property
    def interfaces(self):
        """ Indexed interfaces state """
        return self._ifaces_state

    @property
    def routes(self):
        """
        Return configured routes
        """
        return self._state.get(Route.KEY, {}).get(Route.CONFIG, [])

    @property
    def iface_routes(self):
        """
        Indexed routes by next hop interface name. Read only.
        """
        return State._index_routes_by_iface(self.routes)

    def sanitize_ethernet(self, other_state):
        """
        Given the other_state, update the ethernet interfaces state base on
        the other_state ethernet interfaces data.
        Usually the other_state represents the current state.
        If auto-negotiation, speed and duplex settings are not provided,
        but exist in the current state, they need to be set to None
        to not override them with the values from the current settings
        since the current settings are read from the device state and not
        from the actual configuration.  This makes it possible to distinguish
        whether a user specified these values in the later configuration step.
        """
        for ifname, iface_state in six.viewitems(self.interfaces):
            iface_current_state = other_state.interfaces.get(ifname, {})
            if iface_current_state.get('type') == 'ethernet':
                ethernet = iface_state.setdefault('ethernet', {})
                ethernet.setdefault('auto-negotiation', None)
                ethernet.setdefault('speed', None)
                ethernet.setdefault('duplex', None)

    def sanitize_dynamic_ip(self):
        """
        If dynamic IP is enabled and IP address is missing, set an empty
        address list. This assures that the desired state is not complemented
        by the current state address values.
        If dynamic IP is disabled, all dynamic IP options should be removed.
        """
        for iface_state in six.viewvalues(self.interfaces):
            for family in ('ipv4', 'ipv6'):
                ip = iface_state.get(family, {})
                if ip.get('enabled') and (
                        ip.get('dhcp') or ip.get('autoconf')):
                    ip['address'] = []
                else:
                    for dhcp_option in ('auto-routes',
                                        'auto-gateway',
                                        'auto-dns'):
                        ip.pop(dhcp_option, None)

    def verify_interfaces(self, other_state):
        """Verify that the (self) state is a subset of the other_state. """
        self._remove_absent_interfaces()
        self._remove_down_virt_interfaces()

        self._assert_interfaces_included_in(other_state)

        metadata.remove_ifaces_metadata(self)
        other_state.sanitize_dynamic_ip()

        self.merge_interfaces(other_state)

        self.normalize_for_verification()
        other_state.normalize_for_verification()

        self._assert_interfaces_equal(other_state)

    def verify_routes(self, other_state):
        """
        Verify that the self state and the other_state are identical.
        """
        self._clean_routes()
        other_state._clean_routes()
        for iface_name, routes in six.viewitems(self.iface_routes):
            routes.sort(key=_route_sort_key)
            other_routes = other_state.iface_routes.get(iface_name, [])
            other_routes.sort(key=_route_sort_key)
            if routes != other_routes:
                raise NmstateVerificationError(
                    format_desired_current_state_diff(
                        {
                            Route.KEY: routes
                        },
                        {
                            Route.KEY: other_routes
                        }))

    def normalize_for_verification(self):
        self._clean_sanitize_ethernet()
        self._sort_lag_slaves()
        self._sort_bridge_ports()
        self._canonicalize_ipv6()
        self._remove_iface_ipv6_link_local_addr()
        self._sort_ip_addresses()
        self._capitalize_mac()

    def merge_interfaces(self, other_state):
        """
        Given the self and other states, complete the self state by merging
        the missing parts from the current state.
        The operation is performed on entries that exist in both states,
        entries that appear only on one state are ignored.
        This is a reverse recursive update operation.
        """
        other_state = State(other_state.state)
        for name in (six.viewkeys(self.interfaces) &
                     six.viewkeys(other_state.interfaces)):
            dict_update(other_state.interfaces[name], self.interfaces[name])
            self._ifaces_state[name] = other_state.interfaces[name]

    def merge_route_config(self, current):
        """
        Merge routes from current to self.
        If any changed route referring non-exist interface, create the
        interface state with name only, in order to trigger the recreating of
        the interface profile.
        Delete route entries if 'state: absent'.
        """
        # Merge other_routes
        iface_route_sets = defaultdict(set)
        absent_route_sets = set()
        current_iface_route_sets = defaultdict(set)

        for route in self.routes:
            if route.get(Route.STATE) == Route.ABSENT:
                absent_route_sets.add(_Route(route))
            else:
                if Route.NEXT_HOP_INTERFACE in route:
                    iface_route_sets[route[Route.NEXT_HOP_INTERFACE]].add(
                        _Route(route))
                else:
                    logging.warning("Ignoring the route entry with no "
                                    "next hop interface defined: %s",
                                    route)

        for route in current.routes:
            iface_route_sets[route[Route.NEXT_HOP_INTERFACE]].add(
                _Route(route))
            current_iface_route_sets[route[Route.NEXT_HOP_INTERFACE]].add(
                _Route(route))

        change_ifaces = _remove_absent_routes(absent_route_sets,
                                              iface_route_sets,
                                              current_iface_route_sets)

        for iface_name in list(six.viewkeys(iface_route_sets)):
            # Remove routes if certain interface routes never changes.
            if iface_route_sets[iface_name] == \
               current_iface_route_sets.get(iface_name):
                del iface_route_sets[iface_name]

        # Create basic interface information for changed routes.
        for iface_name in set(six.viewkeys(iface_route_sets)) | change_ifaces:
            if iface_name not in self.interfaces and \
               iface_name in current.interfaces:
                self.interfaces[iface_name] = {'name': iface_name}

        self._state[Route.KEY] = {Route.CONFIG: []}
        for iface_name, route_set in six.viewitems(iface_route_sets):
            self._state[Route.KEY][Route.CONFIG].extend(
                list(route_obj.state for route_obj in route_set))

    def _remove_absent_interfaces(self):
        ifaces = {}
        for ifname, ifstate in six.viewitems(self.interfaces):
            is_absent = ifstate.get('state') == 'absent'
            if not is_absent:
                ifaces[ifname] = ifstate
        self._ifaces_state = ifaces

    def _remove_down_virt_interfaces(self):
        ifaces = {}
        for ifname, ifstate in six.viewitems(self.interfaces):
            is_virt_down = (
                ifstate.get('state') == 'down' and
                ifstate.get('type') in InterfaceType.VIRT_TYPES
            )
            if not is_virt_down:
                ifaces[ifname] = ifstate
        self._ifaces_state = ifaces

    @staticmethod
    def _index_interfaces_state_by_name(state):
        return {iface['name']: iface for iface in state.get(Interface.KEY, [])}

    @staticmethod
    def _index_routes_by_iface(routes):
        iface_routes = defaultdict(list)
        for route in routes:
            if Route.NEXT_HOP_INTERFACE in route:
                iface_routes[route[Route.NEXT_HOP_INTERFACE]].append(route)
        return iface_routes

    def _clean_sanitize_ethernet(self):
        for ifstate in six.viewvalues(self.interfaces):
            ethernet_state = ifstate.get('ethernet')
            if ethernet_state:
                for key in ('auto-negotiation', 'speed', 'duplex'):
                    if ethernet_state.get(key, None) is None:
                        ethernet_state.pop(key, None)
                if not ethernet_state:
                    ifstate.pop('ethernet', None)

    def _clean_routes(self):
        """
        Remove routes for down/absent interface
        Remove routes for non-exit interface
        Remove routes when IPv4/IPv6 down.
        """
        self._clean_routes_iface_down()
        self._clean_routes_missing_iface()
        self._clean_routes_ip_down()

    def _clean_routes_iface_down(self):
        for iface_name, iface_state in six.viewitems(self.interfaces):
            if iface_state[Interface.STATE] in (InterfaceState.DOWN,
                                                InterfaceState.ABSENT):
                if self.iface_routes.get(iface_name, []):
                    logging.info(
                        "Removing routes next hop to interface %s "
                        "which is in %s state", iface_name,
                        iface_state[Interface.STATE])
                    for route in self.iface_routes[iface_name]:
                        self.routes.remove(route)

    def _clean_routes_missing_iface(self):
        for missing_iface in set(self.iface_routes) - set(self.interfaces):
            if self.iface_routes.get(missing_iface, []):
                logging.info(
                    "Removing routes next hop to interface %s "
                    "which does not exists", missing_iface)
                for route in self.iface_routes[missing_iface]:
                    self.routes.remove(route)

    def _clean_routes_ip_down(self):
        removed_iface_family = set()
        for route in self.routes:
            iface_name = route[Route.NEXT_HOP_INTERFACE]
            iface_state = self.interfaces[iface_name]
            if iplib.is_ipv6_address(route[Route.DESTINATION]):
                if not iface_state.get('ipv6', {}).get('enabled'):
                    removed_iface_family.add((iface_name, 'ipv6'))
                    self.routes.remove(route)
            else:
                if not iface_state.get('ipv4', {}).get('enabled'):
                    removed_iface_family.add((iface_name, 'ipv4'))
                    self.routes.remove(route)

        for iface_name, family in removed_iface_family:
            logging.info(
                "Removing %s routes next hop to interface %s "
                "which has %s disabled", family, iface_name, family)

    def _sort_lag_slaves(self):
        for ifstate in six.viewvalues(self.interfaces):
            ifstate.get('link-aggregation', {}).get('slaves', []).sort()

    def _sort_bridge_ports(self):
        for ifstate in six.viewvalues(self.interfaces):
            ifstate.get('bridge', {}).get('port', []).sort(
                key=itemgetter('name'))

    def _canonicalize_ipv6(self):
        for ifstate in six.viewvalues(self.interfaces):
            new_state = {Interface.IPV6: {'enabled': False, 'address': []}}
            dict_update(new_state, ifstate)
            self._ifaces_state[ifstate[Interface.NAME]] = new_state

    def _remove_iface_ipv6_link_local_addr(self):
        for ifstate in six.viewvalues(self.interfaces):
            ifstate['ipv6']['address'] = list(
                addr for addr in ifstate['ipv6']['address']
                if not iplib.is_ipv6_link_local_addr(addr['ip'],
                                                     addr['prefix-length'])
            )

    def _sort_ip_addresses(self):
        for ifstate in six.viewvalues(self.interfaces):
            for family in ('ipv4', 'ipv6'):
                ifstate.get(family, {}).get('address', []).sort(
                    key=itemgetter('ip'))

    def _capitalize_mac(self):
        for ifstate in six.viewvalues(self.interfaces):
            mac = ifstate.get(Interface.MAC)
            if mac:
                ifstate[Interface.MAC] = mac.upper()

    def _assert_interfaces_equal(self, current_state):
        for ifname in self.interfaces:
            iface_dstate = self.interfaces[ifname]
            iface_cstate = current_state.interfaces[ifname]

            if iface_dstate != iface_cstate:
                raise NmstateVerificationError(
                    format_desired_current_state_diff(
                        self.interfaces[ifname],
                        current_state.interfaces[ifname]
                    )
                )

    def _assert_interfaces_included_in(self, current_state):
        if not (set(self.interfaces) <= set(
                current_state.interfaces)):
            raise NmstateVerificationError(
                format_desired_current_state_diff(self.interfaces,
                                                  current_state.interfaces))


def dict_update(origin_data, to_merge_data):
    """Recursevely performes a dict update (merge)"""

    for key, val in six.viewitems(to_merge_data):
        if isinstance(val, Mapping):
            origin_data[key] = dict_update(origin_data.get(key, {}), val)
        else:
            origin_data[key] = val
    return origin_data


def _route_sort_key(route):
    return (route.get(Route.TABLE_ID, -1),
            route.get(Route.NEXT_HOP_INTERFACE, ''),
            route.get(Route.DESTINATION, ''))


def _remove_absent_routes(absent_route_sets, iface_route_sets,
                          current_iface_route_sets):
    """
    Remove routes based on absent routes:
        * Treat missing property as wildcard match.
        * Mark interface as edited(include it in self.interfaces) if
          routes changed.
    Return a list of interface names got route deleted.
    """
    changed_ifaces = set()
    for absent_route in absent_route_sets:
        for iface_name, route_set in six.viewitems(iface_route_sets):
            if Route.NEXT_HOP_INTERFACE in absent_route.state and \
               absent_route.state[Route.NEXT_HOP_INTERFACE] != iface_name:
                continue
            for route in copy.deepcopy(route_set):
                if absent_route.is_match(route):
                    # Make sure this route entry is copied, not added by user.
                    if not iface_name:
                        iface_name = route.state[Route.NEXT_HOP_INTERFACE]
                    current_route_set = current_iface_route_sets.get(
                        iface_name)
                    if route in current_route_set:
                        changed_ifaces.add(iface_name)
                        route_set.remove(route)
    return changed_ifaces
