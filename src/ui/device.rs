use crate::astrobox::psys_host::register;

pub async fn ensure_interconnect_registered(device_addr: &str) {
    let _ = register::register_interconnect_recv(
        device_addr,
        "com.yzf.daymatter",
    )
    .await;
}
