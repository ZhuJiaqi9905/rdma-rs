use rdma_rs::ibv::IbvContext;

fn main() {
    println!("Hello, world!");
    let cxt = IbvContext::new(Some("mlx5_1")).unwrap();
    let attr = cxt.query_device().unwrap();
    println!("{}", attr.fw_ver())
}
