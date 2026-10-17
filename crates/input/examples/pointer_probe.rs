//! 本机排障用：创建注入器并把指针右移三次，看 X 侧有没有动。
fn main() {
    let mut inj = halokey_input::open_default().expect("open injector");
    std::thread::sleep(std::time::Duration::from_millis(1500));
    for _ in 0..3 {
        inj.pointer_move(200, 0).expect("pointer_move");
        std::thread::sleep(std::time::Duration::from_millis(150));
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
}
