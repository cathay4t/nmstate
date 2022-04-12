use crate::nm::nm_dbus::{NmConnection, NmSettingEthtool};
use crate::{
    EthtoolCoalesceConfig, EthtoolFeatureConfig, EthtoolPauseConfig,
    EthtoolRingConfig, Interface,
};

pub(crate) fn gen_ethtool_setting(
    iface: &Interface,
    nm_conn: &mut NmConnection,
) {
    if let Some(ethtool_iface) = iface.base_iface().ethtool.as_ref() {
        let mut nm_ethtool_set =
            nm_conn.ethtool.as_ref().cloned().unwrap_or_default();

        if let Some(pause_conf) = ethtool_iface.pause.as_ref() {
            apply_pause_options(&mut nm_ethtool_set, pause_conf);
        }
        if let Some(feature_conf) = ethtool_iface.feature.as_ref() {
            apply_feature_options(&mut nm_ethtool_set, feature_conf);
        }
        if let Some(coalesce_conf) = ethtool_iface.coalesce.as_ref() {
            apply_coalesce_options(&mut nm_ethtool_set, coalesce_conf);
        }
        if let Some(ring_conf) = ethtool_iface.ring.as_ref() {
            apply_ring_options(&mut nm_ethtool_set, ring_conf);
        }

        nm_conn.ethtool = Some(nm_ethtool_set);
    }
}

fn apply_pause_options(
    nm_ethtool_set: &mut NmSettingEthtool,
    pause_conf: &EthtoolPauseConfig,
) {
    nm_ethtool_set.pause_rx = pause_conf.rx;
    nm_ethtool_set.pause_tx = pause_conf.tx;
    nm_ethtool_set.pause_autoneg = pause_conf.autoneg;
}

fn apply_feature_options(
    nm_ethtool_set: &mut NmSettingEthtool,
    feature_conf: &EthtoolFeatureConfig,
) {
    nm_ethtool_set.feature_rx = feature_conf.rx_checksum;
    nm_ethtool_set.feature_gro = feature_conf.rx_gro;
    nm_ethtool_set.feature_lro = feature_conf.rx_lro;
    nm_ethtool_set.feature_rxvlan = feature_conf.rx_vlan_hw_parse;
    nm_ethtool_set.feature_txvlan = feature_conf.tx_vlan_hw_insert;
    nm_ethtool_set.feature_ntuple = feature_conf.rx_ntuple_filter;
    nm_ethtool_set.feature_rxhash = feature_conf.rx_hashing;
    nm_ethtool_set.feature_sg = feature_conf.tx_scatter_gather;
    nm_ethtool_set.feature_tso = feature_conf.tx_tcp_segmentation;
    nm_ethtool_set.feature_tso = feature_conf.tx_generic_segmentation;
    nm_ethtool_set.feature_highdma = feature_conf.highdma
}

fn apply_coalesce_options(
    nm_ethtool_set: &mut NmSettingEthtool,
    coalesce_conf: &EthtoolCoalesceConfig,
) {
    nm_ethtool_set.coalesce_adaptive_rx = coalesce_conf.adaptive_rx;
    nm_ethtool_set.coalesce_adaptive_tx = coalesce_conf.adaptive_tx;
    nm_ethtool_set.coalesce_pkt_rate_high = coalesce_conf.pkt_rate_high;
    nm_ethtool_set.coalesce_pkt_rate_low = coalesce_conf.pkt_rate_low;
    nm_ethtool_set.coalesce_rx_frames = coalesce_conf.rx_frames;
    nm_ethtool_set.coalesce_rx_frames_high = coalesce_conf.rx_frames_high;
    nm_ethtool_set.coalesce_rx_frames_low = coalesce_conf.rx_frames_low;
    nm_ethtool_set.coalesce_rx_frames_irq = coalesce_conf.rx_frames_irq;
    nm_ethtool_set.coalesce_tx_frames = coalesce_conf.tx_frames;
    nm_ethtool_set.coalesce_tx_frames_high = coalesce_conf.tx_frames_high;
    nm_ethtool_set.coalesce_tx_frames_low = coalesce_conf.tx_frames_low;
    nm_ethtool_set.coalesce_tx_frames_irq = coalesce_conf.tx_frames_irq;
    nm_ethtool_set.coalesce_rx_usecs = coalesce_conf.rx_usecs;
    nm_ethtool_set.coalesce_rx_usecs_high = coalesce_conf.rx_usecs_high;
    nm_ethtool_set.coalesce_rx_usecs_low = coalesce_conf.rx_usecs_low;
    nm_ethtool_set.coalesce_rx_usecs_irq = coalesce_conf.rx_usecs_irq;
    nm_ethtool_set.coalesce_sample_interval = coalesce_conf.sample_interval;
    nm_ethtool_set.coalesce_stats_block_usecs = coalesce_conf.stats_block_usecs;
}

fn apply_ring_options(
    nm_ethtool_set: &mut NmSettingEthtool,
    ring_conf: &EthtoolRingConfig,
) {
    nm_ethtool_set.ring_rx = ring_conf.rx;
    nm_ethtool_set.ring_rx_jumbo = ring_conf.rx_jumbo;
    nm_ethtool_set.ring_rx_mini = ring_conf.rx_mini;
    nm_ethtool_set.ring_tx = ring_conf.tx;
}
