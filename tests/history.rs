//! `Solid::iter_history` population tests (#156).
//!
//! history は「派生系譜」: 結果に生き残った各 face を同種(face)の元 face に
//! 対応付けた flat `[post_id, src_id]` 列。identity(pass-through) ∪ Modified を
//! 含み、Generated(edge→face の新規面) は含まない。詳細は
//! notes/20260603-history定義の明確化.md を参照。

use cadrum::{DVec3, Edge, ProfileOrient, Solid};
use std::collections::HashSet;
use std::f64::consts::TAU;

/// 一辺 `side` の閉じた正方形プロファイル（extrude 用）。
fn square(side: f64) -> Vec<Edge> {
	Edge::polygon(&[DVec3::new(0.0, 0.0, 0.0), DVec3::new(side, 0.0, 0.0), DVec3::new(side, side, 0.0), DVec3::new(0.0, side, 0.0)]).expect("square polygon")
}

/// 入力に face を持たない演算（プリミティブ / edge・grid ソースの builder）は
/// history が空のまま（保持元 face が無いので by design）。
#[test]
fn test_no_face_source_ops_have_empty_history() {
	assert_eq!(Solid::cube(DVec3::ZERO, DVec3::splat(1.0)).iter_history().count(), 0, "cube");
	assert_eq!(Solid::sphere(1.0).iter_history().count(), 0, "sphere");

	let extruded = Solid::extrude(&square(4.0), DVec3::Z * 3.0).expect("extrude");
	assert_eq!(extruded.iter_history().count(), 0, "extrude");

	let profile = [Edge::circle(1.0, DVec3::Z).expect("circle")];
	let spine = [Edge::line(DVec3::ZERO, DVec3::Z * 5.0).expect("line")];
	let swept = Solid::sweep(&profile, &spine, ProfileOrient::Fixed).expect("sweep");
	assert_eq!(swept.iter_history().count(), 0, "sweep");

	let lower = [Edge::circle(3.0, DVec3::Z).expect("circle")];
	let upper = [Edge::circle(1.5, DVec3::Z).expect("circle").translate(DVec3::Z * 8.0)];
	let lofted = Solid::loft(&[lower, upper], false).expect("loft");
	assert_eq!(lofted.iter_history().count(), 0, "loft");

	let bspline = Solid::bspline(16, 8, true, |i, j| {
		let phi = TAU * i as f64 / 16.0;
		let theta = TAU * j as f64 / 8.0;
		let r = 3.0 + theta.cos();
		DVec3::new(r * phi.cos(), r * phi.sin(), theta.sin())
	})
	.expect("bspline torus");
	assert_eq!(bspline.iter_history().count(), 0, "bspline");
}

/// shell: cube の top を開けて内側 offset。残り 5 面は Modified されて outer wall
/// になる。除去された top は Deleted で history に出ない。inner wall は Generated
/// なので出ない。
#[test]
fn test_shell_history_maps_five_retained_faces() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let original: HashSet<u64> = cube.iter_face().map(|f| f.id()).collect();
	let top = cube.iter_face().last().expect("cube has faces");
	let top_id = top.id();
	let shelled = cube.shell(-1.0, [top]).expect("shell");

	let hist: Vec<[u64; 2]> = shelled.iter_history().collect();
	assert!(!hist.is_empty(), "shell must populate history");
	for [_, src] in &hist {
		assert!(original.contains(src), "src {src} is not an original cube face");
	}
	let srcs: HashSet<u64> = hist.iter().map(|[_, s]| *s).collect();
	assert_eq!(srcs.len(), 5, "5 retained faces should map (top removed); got {}", srcs.len());
	assert!(!srcs.contains(&top_id), "removed top face must not appear as src");
}

/// fillet: cube の edge 1 本を fillet → その edge を共有する 2 面が Modified
/// (post != src)。非影響の 4 面は identity (post == src) として現れ、全 6 面が
/// src として登場する（= 無変更面の tshape 保持＝設計前提の検証）。
#[test]
fn test_fillet_history_modifies_adjacent_identity_elsewhere() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let original: HashSet<u64> = cube.iter_face().map(|f| f.id()).collect();
	let edge = cube.iter_edge().next().expect("cube has edges");
	let edge_id = edge.id();
	let adjacent: HashSet<u64> = cube.iter_face().filter(|f| f.iter_edge().any(|e| e.id() == edge_id)).map(|f| f.id()).collect();
	assert_eq!(adjacent.len(), 2, "a cube edge borders exactly 2 faces");

	let filleted = cube.fillet_edges(0.5, [edge]).expect("fillet");
	let hist: Vec<[u64; 2]> = filleted.iter_history().collect();

	for [_, src] in &hist {
		assert!(original.contains(src), "src {src} is not an original cube face");
	}
	let srcs: HashSet<u64> = hist.iter().map(|[_, s]| *s).collect();
	assert_eq!(srcs.len(), 6, "all 6 faces should appear as src (identity preserved); got {}", srcs.len());

	let modified: HashSet<u64> = hist.iter().filter(|[p, s]| p != s).map(|[_, s]| *s).collect();
	for id in &adjacent {
		assert!(modified.contains(id), "adjacent face {id} must be Modified (post != src)");
	}
}

/// chamfer: fillet と同形（edge を共有する 2 面が Modified、全 6 面が src 登場）。
#[test]
fn test_chamfer_history_modifies_adjacent_identity_elsewhere() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let original: HashSet<u64> = cube.iter_face().map(|f| f.id()).collect();
	let edge = cube.iter_edge().next().expect("cube has edges");
	let edge_id = edge.id();
	let adjacent: HashSet<u64> = cube.iter_face().filter(|f| f.iter_edge().any(|e| e.id() == edge_id)).map(|f| f.id()).collect();
	assert_eq!(adjacent.len(), 2, "a cube edge borders exactly 2 faces");

	let chamfered = cube.chamfer_edges(0.5, [edge]).expect("chamfer");
	let hist: Vec<[u64; 2]> = chamfered.iter_history().collect();

	for [_, src] in &hist {
		assert!(original.contains(src), "src {src} is not an original cube face");
	}
	let srcs: HashSet<u64> = hist.iter().map(|[_, s]| *s).collect();
	assert_eq!(srcs.len(), 6, "all 6 faces should appear as src (identity preserved); got {}", srcs.len());

	let modified: HashSet<u64> = hist.iter().filter(|[p, s]| p != s).map(|[_, s]| *s).collect();
	for id in &adjacent {
		assert!(modified.contains(id), "adjacent face {id} must be Modified (post != src)");
	}
}

/// color: fillet 後も Modified/identity 面が src 面の色を history 経由で引き継ぐ。面色を
/// `colormap_mut` で直接置くのは、`Solid::color` がソリッド単位の色を塗るため。
#[cfg(feature = "color")]
#[test]
fn test_fillet_carries_face_color_via_history() {
	let red = cadrum::Color::from_str("#ff0000").expect("valid hex");
	let mut cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let face_ids: Vec<u64> = cube.iter_face().map(|f| f.id()).collect();
	for id in face_ids {
		cube.colormap_mut().insert(id, red);
	}

	let edge = cube.iter_edge().next().expect("cube has edges");
	let filleted = cube.fillet_edges(0.5, [edge]).expect("fillet");

	let hist: Vec<[u64; 2]> = filleted.iter_history().collect();
	assert!(!hist.is_empty(), "fillet must populate history");
	for [post, _] in &hist {
		assert!(filleted.colormap().contains_key(post), "face {post} should inherit color via history");
	}
}

// ==================== edge history (iter_edge_history) ====================

// edge の派生系譜。face history と同形だが `TopAbs_EDGE` 上で: 生き残った各 edge を
// 元 edge に対応付けた `[post_id, src_id]` 列。identity ∪ Modified を含み、Generated
// (新規 edge: fillet/chamfer/section) は含まない。

/// primitive / edge・grid ソースの builder は edge history も空（face history と対称）。
#[test]
fn test_no_source_ops_have_empty_edge_history() {
	assert_eq!(Solid::cube(DVec3::ZERO, DVec3::splat(1.0)).iter_edge_history().count(), 0, "cube");
	assert_eq!(Solid::sphere(1.0).iter_edge_history().count(), 0, "sphere");
	let extruded = Solid::extrude(&square(2.0), DVec3::new(0.0, 0.0, 3.0)).expect("extrude");
	assert_eq!(extruded.iter_edge_history().count(), 0, "extrude");
}

/// fillet: cube の edge 1 本を fillet → その edge は円弧を Generated する形で削除され
/// src に現れない。生き残る edge は identity/Modified、全 post は結果の edge、全 src は
/// 元 cube の edge。
#[test]
fn test_fillet_populates_edge_history() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let original: HashSet<u64> = cube.iter_edge().map(|e| e.id()).collect();
	assert_eq!(original.len(), 12, "cube has 12 edges");

	let edge = cube.iter_edge().next().expect("cube has edges");
	let filleted_edge_id = edge.id();
	let filleted = cube.fillet_edges(0.5, [edge]).expect("fillet");

	let hist: Vec<[u64; 2]> = filleted.iter_edge_history().collect();
	assert!(!hist.is_empty(), "fillet must populate edge history");

	let result_edges: HashSet<u64> = filleted.iter_edge().map(|e| e.id()).collect();
	for [post, src] in &hist {
		assert!(original.contains(src), "src {src} is not an original cube edge");
		assert!(result_edges.contains(post), "post {post} is not an edge of the result");
	}
	let srcs: HashSet<u64> = hist.iter().map(|[_, s]| *s).collect();
	assert!(!srcs.contains(&filleted_edge_id), "the filleted edge is deleted (Generated arc) and must not appear as src");
}

/// boolean: 2 つの重なる cube の交差は、入力 cube の edge を src とする edge history を
/// 生む（cube a の境界 edge が結果へ Modified/trim される）。shallow-copy な
/// `Solid::boolean` が edge の TShape id も保つことの担保でもある。
#[test]
fn test_boolean_populates_edge_history() {
	let a = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let b = Solid::cube(DVec3::splat(5.0), DVec3::splat(15.0));
	let a_edges: HashSet<u64> = a.iter_edge().map(|e| e.id()).collect();

	let inter: Solid = (&a * &b).build().expect("intersection");
	let hist: Vec<[u64; 2]> = inter.iter_edge_history().collect();
	assert!(!hist.is_empty(), "boolean must populate edge history");

	let matched: Vec<u64> = hist.iter().filter(|[_, s]| a_edges.contains(s)).map(|[p, _]| *p).collect();
	assert!(!matched.is_empty(), "at least one result edge must be sourced from cube a");
}

/// Generated edges: fillet erzeugt neue Blend-Kanten, die auf die gefilletete
/// Kante zurückgeführt werden (`iter_generated_edges`). Diese Kanten haben keine
/// Modified-Herkunft — genau die, die ein voll gefilletetes Teil beim Picken zeigt.
#[test]
fn test_fillet_populates_generated_edges() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let edge = cube.iter_edge().next().expect("cube has edges");
	let edge_id = edge.id();
	let filleted = cube.fillet_edges(1.0, [edge]).expect("fillet");

	let gen: Vec<[u64; 2]> = filleted.iter_generated_edges().collect();
	assert!(!gen.is_empty(), "fillet muss Generated-Kanten liefern");
	// Alle gemeldeten Generated-Kanten sind Kanten des Ergebnisses …
	let result_edges: HashSet<u64> = filleted.iter_edge().map(|e| e.id()).collect();
	for [g, _] in &gen {
		assert!(result_edges.contains(g), "gen edge {g} ist eine Ergebnis-Kante");
	}
	// … und die gefilletete Kante taucht als Quelle auf.
	let srcs: HashSet<u64> = gen.iter().map(|[_, s]| *s).collect();
	assert!(srcs.contains(&edge_id), "die gefilletete Kante ist Quelle der Blend-Kanten");
}

/// extrude: die **Geburt** eines Kanten-Namens. extrude baut die Topologie
/// komplett neu (edge history bleibt leer), aber jede Ergebnis-Kante ist
/// `Generated()` aus genau der Profil-Kante, deren Mantelfläche sie berandet —
/// damit lässt sich eine Kante auf das Skizzen-Segment zurückführen, aus dem
/// sie gewachsen ist, statt sie geometrisch zu suchen.
#[test]
fn test_extrude_populates_generated_edges_from_profile() {
	let profile = square(4.0);
	let profile_ids: HashSet<u64> = profile.iter().map(|e| e.id()).collect();
	assert_eq!(profile_ids.len(), 4, "Quadrat-Profil hat 4 Kanten");

	let extruded = Solid::extrude(&profile, DVec3::Z * 3.0).expect("extrude");
	let gen: Vec<[u64; 2]> = extruded.iter_generated_edges().collect();
	assert!(!gen.is_empty(), "extrude muss Generated-Kanten liefern");

	// Jede gemeldete Kante gehört zum Ergebnis, jede Quelle ist eine Profil-Kante.
	let result_edges: HashSet<u64> = extruded.iter_edge().map(|e| e.id()).collect();
	for [g, s] in &gen {
		assert!(result_edges.contains(g), "gen edge {g} ist eine Ergebnis-Kante");
		assert!(profile_ids.contains(s), "src {s} ist eine Profil-Kante");
	}
	// Alle vier Segmente sind wiederfindbar (kein Segment fällt durch).
	let srcs: HashSet<u64> = gen.iter().map(|[_, s]| *s).collect();
	assert_eq!(srcs, profile_ids, "jedes Profil-Segment muss als Quelle auftauchen");

	// Die Mantelfläche eines Segments wird von 4 Kanten berandet (Boden-, Deckel-
	// und zwei senkrechte Kanten); die senkrechten teilen sich zwei Nachbarn.
	for id in &profile_ids {
		let n = gen.iter().filter(|[_, s]| s == id).count();
		assert_eq!(n, 4, "Segment {id} berandet seine Mantelfläche mit 4 Kanten, hat {n}");
	}
	assert_eq!(gen.len(), 16, "4 Segmente × 4 Kanten = 16 Paare");

	// **Die eigentliche Zusage**: es gibt keine namenlose Kante — jede Kante des
	// Ergebnisses ist auf ein Profil-Segment zurückführbar. (Ids liegen unter der
	// Zahl der Kanten, weil Boden- und Deckelkopie einer Profil-Kante dasselbe
	// TShape teilen; siehe `subshape_id` in cpp/wrapper.cpp — für die Herkunft
	// harmlos, denn beide stammen aus demselben Segment.)
	let covered: HashSet<u64> = gen.iter().map(|[g, _]| *g).collect();
	let all: HashSet<u64> = extruded.iter_edge().map(|e| e.id()).collect();
	assert!(all.difference(&covered).next().is_none(), "jede Ergebnis-Kante muss eine Herkunft haben, ohne: {:?}", all.difference(&covered).collect::<Vec<_>>());
}

/// share(): flacher Copy teilt das TShape → edge/face id と history が保存される
/// (deep-copy な Clone と違い、provenance マッチが copy を跨いで成立する)。
#[test]
fn test_share_preserves_ids_and_history() {
	// history 付き solid (fillet 結果) を share しても edge id と edge history が一致。
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let edge = cube.iter_edge().next().expect("cube has edges");
	let filleted = cube.fillet_edges(0.5, [edge]).expect("fillet");

	let shared = filleted.share();
	let orig_edges: HashSet<u64> = filleted.iter_edge().map(|e| e.id()).collect();
	let shared_edges: HashSet<u64> = shared.iter_edge().map(|e| e.id()).collect();
	assert_eq!(orig_edges, shared_edges, "share() must preserve edge ids");

	let orig_hist: Vec<[u64; 2]> = filleted.iter_edge_history().collect();
	let shared_hist: Vec<[u64; 2]> = shared.iter_edge_history().collect();
	assert_eq!(orig_hist, shared_hist, "share() must preserve edge history");

	// Clone (deep-copy) は逆に id を変える — 対比。
	let cloned = filleted.clone();
	let cloned_edges: HashSet<u64> = cloned.iter_edge().map(|e| e.id()).collect();
	assert!(cloned_edges.is_disjoint(&orig_edges), "Clone deep-copies → fresh ids (contrast to share)");
}
