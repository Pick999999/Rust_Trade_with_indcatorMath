use chrono::{Utc, FixedOffset};
use std::env;

fn main() {
    // ดึงเวลาปัจจุบัน โดยบังคับเป็นเวลาไทย (UTC+7) เสมอ
    let tz_offset = FixedOffset::east_opt(7 * 3600).unwrap();
    let now = Utc::now().with_timezone(&tz_offset);
    
    // ตั้งรูปแบบเวลาที่ต้องการ (เช่น 2026-07-13 15:30:00)
    let build_time = now.format("%Y-%m-%d %H:%M:%S").to_string();
    
    // ดึงเวอร์ชันของ rustc
    let _rustc_version = env::var("RUSTC_VERSION").unwrap_or_else(|_| "Unknown".to_string());
    // (หรือจะใช้ Command::new("rustc").arg("-V").output() ก็ได้)
    
    // ส่งข้อมูลไปเป็น Environment Variable สำหรับตอน Compile
    println!("cargo:rustc-env=BUILD_TIME={}", build_time);
    println!("cargo:rustc-env=BUILD_PROFILE={}", env::var("PROFILE").unwrap_or_else(|_| "debug".to_string()));
    
    // บังคับให้ Cargo รัน build.rs ใหม่ทุกครั้งที่มีการกด Build
    // ถ้าต้องการให้ Build cache ทำงานปกติ ลบบรรทัดนี้ออกได้ครับ
    println!("cargo:rerun-if-changed=build.rs");
}
