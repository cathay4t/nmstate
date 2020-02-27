#
# Copyright (c) 2018-2020 Red Hat, Inc.
#
# This file is part of nmstate
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Lesser General Public License as published by
# the Free Software Foundation, either version 2.1 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Lesser General Public License for more details.
#
# You should have received a copy of the GNU Lesser General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.
#

from libnmstate import validator
from libnmstate.schema import DNS
from libnmstate.schema import Interface
from libnmstate.schema import Route
from libnmstate.schema import RouteRule


def show_with_plugin(nm_plugin, include_status_data=None):
    report = {}
    if include_status_data:
        report["capabilities"] = nm_plugin.capabilities

    report[Interface.KEY] = nm_plugin.get_interfaces()
    report[Route.KEY] = nm_plugin.get_routes()
    report[RouteRule.KEY] = nm_plugin.get_route_rules()
    report[DNS.KEY] = nm_plugin.get_dns_client_config()

    validator.validate(report)
    return report
