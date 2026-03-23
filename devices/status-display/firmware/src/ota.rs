use std::io::{Read, Write};
use std::net::TcpStream;

pub fn handle_update(mut stream: TcpStream) -> anyhow::Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(60)))?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(10)))?;
    stream.set_nodelay(true)?;

    let mut hdr = [0u8; 4];
    stream.read_exact(&mut hdr)?;
    let fw_size = u32::from_le_bytes(hdr) as usize;
    log::info!("OTA: firmware size = {} bytes", fw_size);

    if fw_size == 0 || fw_size > 2 * 1024 * 1024 {
        stream.write_all(b"ER")?;
        anyhow::bail!("OTA: invalid firmware size {}", fw_size);
    }

    unsafe {
        let running = esp_idf_sys::esp_ota_get_running_partition();
        if !running.is_null() {
            log::info!("OTA: running from partition at 0x{:x}", (*running).address);
        }
        let partition = esp_idf_sys::esp_ota_get_next_update_partition(std::ptr::null());
        if partition.is_null() {
            stream.write_all(b"ER")?;
            anyhow::bail!("OTA: no update partition found");
        }
        log::info!("OTA: target partition at 0x{:x}, size 0x{:x}", (*partition).address, (*partition).size);

        let mut handle: esp_idf_sys::esp_ota_handle_t = 0;
        log::info!("OTA: calling esp_ota_begin with OTA_SIZE_UNKNOWN...");
        let ret = esp_idf_sys::esp_ota_begin(partition, esp_idf_sys::OTA_SIZE_UNKNOWN as usize, &mut handle);
        if ret != esp_idf_sys::ESP_OK as i32 {
            stream.write_all(b"ER")?;
            anyhow::bail!("OTA: esp_ota_begin failed: {}", ret);
        }
        log::info!("OTA: flash ready");
        stream.write_all(b"OK")?;

        let mut buf = [0u8; 4096];
        let mut received: usize = 0;

        while received < fw_size {
            let want = std::cmp::min(4096, fw_size - received);
            let n = stream.read(&mut buf[..want])?;
            if n == 0 {
                esp_idf_sys::esp_ota_abort(handle);
                anyhow::bail!("OTA: connection closed at {} / {}", received, fw_size);
            }
            let ret = esp_idf_sys::esp_ota_write(handle, buf.as_ptr() as *const _, n);
            if ret != esp_idf_sys::ESP_OK as i32 {
                esp_idf_sys::esp_ota_abort(handle);
                anyhow::bail!("OTA: esp_ota_write failed: {}", ret);
            }
            received += n;
            let pct = received * 100 / fw_size;
            if pct % 10 == 0 {
                log::info!("OTA: {}%", pct);
            }
        }

        let ret = esp_idf_sys::esp_ota_end(handle);
        if ret != esp_idf_sys::ESP_OK as i32 {
            anyhow::bail!("OTA: esp_ota_end failed: {}", ret);
        }

        let ret = esp_idf_sys::esp_ota_set_boot_partition(partition);
        if ret != esp_idf_sys::ESP_OK as i32 {
            anyhow::bail!("OTA: set_boot_partition failed: {}", ret);
        }

        stream.write_all(b"DN")?;
        log::info!("OTA: rebooting...");
        std::thread::sleep(std::time::Duration::from_millis(200));
        esp_idf_sys::esp_restart();
    }
}
