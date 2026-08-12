//! The BRep reader must ignore bytes past its own payload — that is what lets the
//! color trailer live there.

use cadrum::{DVec3, Solid};

fn test_box() -> Vec<Solid> {
	vec![Solid::cube(DVec3::ZERO, DVec3::ONE)]
}

#[test]
fn read_brep_with_trailing_garbage() {
	let shape = test_box();

	// Write valid BRep binary
	let mut buf = Vec::new();
	cadrum::Solid::write_brep(&shape, &mut buf).expect("write_brep should succeed");
	let brep_len = buf.len();
	assert!(brep_len > 0);

	// Append 1 KB of garbage
	buf.extend_from_slice(&[0xAB; 1024]);
	assert_eq!(buf.len(), brep_len + 1024);

	// Read back — should succeed despite trailing garbage
	let result = cadrum::Solid::read_brep(&mut buf.as_slice());
	match &result {
		Ok(solids) => {
			assert!(!solids.is_empty(), "should read at least one solid");
			let vol = solids[0].volume();
			assert!((vol - 1.0).abs() < 1e-6, "unit box volume should be ~1.0, got {}", vol);
			println!("BINARY: OK — read {} solid(s), volume={:.6}, brep_len={}, total={}", solids.len(), vol, brep_len, brep_len + 1024);
		}
		Err(e) => {
			println!("FAILED — {:?} (brep_len={}, total={})", e, brep_len, brep_len + 1024);
			panic!("read_brep failed with trailing garbage: {:?}", e);
		}
	}
}

/// The reader picks its flavour from the header, and the two magics nest: the binary
/// one CONTAINS the ASCII one. This pins the trap, because a sniff that looks for the
/// short magic first routes every binary file into the ASCII reader — which rejects it
/// with "File was not written with this version of the topology".
#[test]
fn the_binary_header_contains_the_ascii_magic() {
	let mut buf = Vec::new();
	cadrum::Solid::write_brep(&test_box(), &mut buf).expect("write_brep should succeed");
	let head = String::from_utf8_lossy(&buf[..64.min(buf.len())]).into_owned();

	assert!(head.contains("Open CASCADE Topology V"), "binary magic moved: {head:?}");
	assert!(head.contains("CASCADE Topology V"), "the ASCII magic is a substring of it: {head:?}");
}
