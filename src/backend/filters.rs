use std::collections::{HashMap, HashSet};

use anyhow::Result;
use tokio::sync::{OnceCell, RwLock};

static FILTERS: OnceCell<RwLock<HashMap<DevEui, HashSet<DevAddr>>>> = OnceCell::const_new();

pub type DevEui = [u8; 8];
pub type DevAddr = [u8; 4];

pub async fn setup() -> Result<()> {
    FILTERS
        .set(RwLock::new(HashMap::new()))
        .map_err(|e| anyhow!("OnceCell error: {}", e))
}

pub async fn remove_dev_eui(dev_eui: DevEui) {
    if let Some(f) = FILTERS.get() {
        let mut f_lock = f.write().await;
        f_lock.remove(&dev_eui);
    }
}

pub async fn set_dev_eui(dev_eui: DevEui, dev_addrs: HashSet<DevAddr>) {
    if let Some(f) = FILTERS.get() {
        let mut f_lock = f.write().await;
        f_lock.insert(dev_eui, dev_addrs);
    }
}

pub async fn flush() {
    if let Some(f) = FILTERS.get() {
        let mut f_lock = f.write().await;
        *f_lock = HashMap::new();
    }
}

pub async fn get() -> HashMap<DevEui, HashSet<DevAddr>> {
    if let Some(f) = FILTERS.get() {
        let f_lock = f.read().await;
        f_lock.clone()
    } else {
        Default::default()
    }
}

pub async fn set(new_filters: HashMap<DevEui, HashSet<DevAddr>>) {
    if let Some(f) = FILTERS.get() {
        let mut f_lock = f.write().await;
        *f_lock = new_filters;
    }
}

pub async fn matches(phy_payload: &[u8]) -> bool {
    if phy_payload.is_empty() {
        return true;
    }

    let mhdr = phy_payload[0];
    let m_type = mhdr >> 5;

    let dev_addr: Option<u32> = match m_type {
        // DataUp
        0x02 | 0x04 => {
            // MHDR + DevAddr
            // [1]    [4]
            if phy_payload.len() >= 5 {
                let mut dev_addr: [u8; 4] = [0; 4];
                dev_addr.clone_from_slice(&phy_payload[1..5]);
                Some(u32::from_le_bytes(dev_addr))
            } else {
                None
            }
        }
        _ => None,
    };

    let dev_eui: Option<u64> = match m_type {
        // JoinRequest
        0x00 => {
            // MHDR + JoinEUI + DevEUI
            // [1]    [8]       [8]
            if phy_payload.len() >= 17 {
                let mut join_eui: [u8; 8] = [0; 8];
                join_eui.clone_from_slice(&phy_payload[9..17]);
                Some(u64::from_le_bytes(join_eui))
            } else {
                None
            }
        }
        _ => None,
    };

    // We could not extract the DevAddr or JoinEUI from the PhyPayload. In this case we let the
    // message pass.
    if dev_addr.is_none() && dev_eui.is_none() {
        return true;
    }

    if let Some(f) = FILTERS.get() {
        let f_lock = f.read().await;

        // Let everything pass on empty filter list.
        if f_lock.is_empty() {
            return true;
        }

        // If we extracted a DevEUI from the PHYPayload, check if this
        // DevEUI is in the filter list.
        if let Some(dev_eui) = dev_eui {
            return f_lock
                .keys()
                .map(|b| u64::from_be_bytes(*b))
                .collect::<Vec<u64>>()
                .contains(&dev_eui);
        }

        // If we extracted a DevAddr from the PHYPayload, check if this
        // DevAddr is in the filter list (under one of the DevEUI keys).
        if let Some(dev_addr) = dev_addr {
            return f_lock
                .values()
                .map(|v| {
                    v.iter()
                        .map(|b| u32::from_be_bytes(*b))
                        .collect::<Vec<u32>>()
                        .contains(&dev_addr)
                })
                .collect::<Vec<bool>>()
                .contains(&true);
        }
    }

    false
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::sync::Mutex;

    static LOCK: Mutex<()> = Mutex::const_new(());

    const PHY_DEV_ADDR_UNC: [u8; 5] = [0x02 << 5, 0x04, 0x03, 0x02, 0x01];
    const PHY_DEV_ADDR_CONF: [u8; 5] = [0x04 << 5, 0x04, 0x03, 0x02, 0x01];

    const PHY_JOIN: [u8; 17] = [
        // MHDR
        0x00, // JoinEUI
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // DevEUI
        0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01,
    ];

    #[tokio::test]
    async fn test_empty_filter_list_passes() {
        let _lock = LOCK.lock().await;

        let _ = setup().await;
        flush().await;

        assert!(matches(&PHY_DEV_ADDR_UNC).await);
        assert!(matches(&PHY_DEV_ADDR_CONF).await);
        assert!(matches(&PHY_JOIN).await);
    }

    #[tokio::test]
    async fn test_join() {
        let _lock = LOCK.lock().await;
        let _ = setup().await;
        flush().await;

        // Does not match DevEUI.
        set_dev_eui([1, 1, 1, 1, 1, 1, 1, 1], HashSet::new()).await;
        assert_eq!(false, matches(&PHY_JOIN).await);

        // Matches DevEUI.
        set_dev_eui([1, 2, 3, 4, 5, 6, 7, 8], HashSet::new()).await;
        assert_eq!(true, matches(&PHY_JOIN).await);
    }

    #[tokio::test]
    async fn test_data_up() {
        let _lock = LOCK.lock().await;
        let _ = setup().await;
        flush().await;

        // Does not match DevAddr.
        set_dev_eui([1, 1, 1, 1, 1, 1, 1, 1], HashSet::from([[1, 1, 1, 1]])).await;
        assert_eq!(false, matches(&PHY_DEV_ADDR_UNC).await);
        assert_eq!(false, matches(&PHY_DEV_ADDR_CONF).await);

        // Does match DevAddr.
        set_dev_eui([1, 1, 1, 1, 1, 1, 1, 1], HashSet::from([[1, 2, 3, 4]])).await;
        assert_eq!(true, matches(&PHY_DEV_ADDR_UNC).await);
        assert_eq!(true, matches(&PHY_DEV_ADDR_CONF).await);
    }

    #[tokio::test]
    async fn test_add_remove() {
        let _lock = LOCK.lock().await;
        let _ = setup().await;
        flush().await;

        // Assert empty list.
        let res = get().await;
        assert!(res.is_empty());

        // Set DevEUI.
        set_dev_eui([1, 2, 3, 4, 5, 6, 7, 8], HashSet::from([[1, 2, 3, 4]])).await;
        let res = get().await;
        assert_eq!(
            HashMap::from([([1, 2, 3, 4, 5, 6, 7, 8], HashSet::from([[1, 2, 3, 4]]))]),
            res
        );

        // Overwrite DevEUI.
        set_dev_eui([1, 2, 3, 4, 5, 6, 7, 8], HashSet::from([[2, 2, 3, 4]])).await;
        let res = get().await;
        assert_eq!(
            HashMap::from([([1, 2, 3, 4, 5, 6, 7, 8], HashSet::from([[2, 2, 3, 4]]))]),
            res
        );

        // Set an other DevEUI.
        set_dev_eui([2, 2, 3, 4, 5, 6, 7, 8], HashSet::from([[1, 2, 3, 4]])).await;
        let res = get().await;
        assert_eq!(
            HashMap::from([
                ([1, 2, 3, 4, 5, 6, 7, 8], HashSet::from([[2, 2, 3, 4]])),
                ([2, 2, 3, 4, 5, 6, 7, 8], HashSet::from([[1, 2, 3, 4]])),
            ]),
            res
        );

        // Remove DevEUI.
        remove_dev_eui([1, 2, 3, 4, 5, 6, 7, 8]).await;
        let res = get().await;
        assert_eq!(
            HashMap::from([([2, 2, 3, 4, 5, 6, 7, 8], HashSet::from([[1, 2, 3, 4]]))]),
            res
        );

        // Flush
        flush().await;
        let res = get().await;
        assert!(res.is_empty());
    }
}
