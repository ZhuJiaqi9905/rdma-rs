extern crate bindgen;

use std::env;
use std::path::PathBuf;

// Adapted from https://rust-lang.github.io/rust-bindgen/tutorial-3.html
fn main() {
    // Tell cargo to tell rustc to link the system bzip2
    // shared library.
    println!("cargo:rustc-link-lib=rdmacm");
    println!("cargo:rustc-link-lib=ibverbs");

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        .bitfield_enum("ibv_device_cap_flags")
        .bitfield_enum("ibv_odp_transport_cap_bits")
        .bitfield_enum("ibv_odp_general_caps")
        .bitfield_enum("ibv_rx_hash_function_flags")
        .bitfield_enum("ibv_rx_hash_fields")
        .bitfield_enum("ibv_raw_packet_caps")
        .bitfield_enum("ibv_tm_cap_flags")
        .bitfield_enum("ibv_pci_atomic_op_size")
        .bitfield_enum("ibv_port_cap_flags")
        .bitfield_enum("ibv_port_cap_flags2")
        .bitfield_enum("ibv_create_cq_wc_flags")
        .bitfield_enum("ibv_wc_flags")
        .bitfield_enum("ibv_access_flags")
        .bitfield_enum("ibv_xrcd_init_attr_mask")
        .bitfield_enum("ibv_rereg_mr_flags")
        .bitfield_enum("ibv_srq_attr_mask")
        .bitfield_enum("ibv_srq_init_attr_mask")
        .bitfield_enum("ibv_wq_init_attr_mask")
        .bitfield_enum("ibv_wq_flags")
        .bitfield_enum("ibv_wq_attr_mask")
        .bitfield_enum("ibv_ind_table_init_attr_mask")
        .bitfield_enum("ibv_qp_init_attr_mask")
        .bitfield_enum("ibv_qp_create_flags")
        .bitfield_enum("ibv_qp_create_send_ops_flags")
        .bitfield_enum("ibv_qp_open_attr_mask")
        .bitfield_enum("ibv_qp_attr_mask")
        .bitfield_enum("ibv_send_flags")
        .bitfield_enum("ibv_ops_flags")
        .bitfield_enum("ibv_cq_attr_mask")
        .bitfield_enum("ibv_flow_flags")
        .bitfield_enum("ibv_flow_action_esp_mask")
        .bitfield_enum("ibv_cq_init_attr_mask")
        .bitfield_enum("ibv_create_cq_attr_flags")
        .bitfield_enum("ibv_parent_domain_init_attr_mask")
        .bitfield_enum("ibv_read_counters_flags")
        .bitfield_enum("ibv_values_mask")
        .bitfield_enum("ib_uverbs_access_flags")
        .bitfield_enum("rdma_cm_join_mc_attr_mask")
        .bitfield_enum("rdma_cm_mc_join_flags")
        .constified_enum_module("ibv_node_type")
        .constified_enum_module("ibv_transport_type")
        .constified_enum_module("ibv_atomic_cap")
        .constified_enum_module("ibv_mtu")
        .constified_enum_module("ibv_port_state")
        .constified_enum_module("ibv_event_type")
        .constified_enum_module("ibv_wc_status")
        .constified_enum_module("ibv_wc_opcode")
        .constified_enum_module("ibv_mw_type")
        .constified_enum_module("ibv_rate")
        .constified_enum_module("ibv_srq_type")
        .constified_enum_module("ibv_wq_type")
        .constified_enum_module("ibv_wq_state")
        .constified_enum_module("ibv_qp_type")
        .constified_enum_module("ibv_qp_state")
        .constified_enum_module("ibv_mig_state")
        .constified_enum_module("ibv_wr_opcode")
        .constified_enum_module("ibv_ops_wr_opcode")
        .constified_enum_module("ibv_flow_attr_type")
        .constified_enum_module("ibv_flow_spec_type")
        .constified_enum_module("ibv_counter_description")
        .constified_enum_module("ibv_rereg_mr_err_code")
        .constified_enum_module("ib_uverbs_advise_mr_advice")
        .constified_enum_module("rdma_cm_event_type")
        .constified_enum_module("rdma_driver_id")
        .constified_enum_module("rdma_port_space")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
