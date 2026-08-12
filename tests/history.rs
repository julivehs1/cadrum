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

/// extrude, eine Dimension höher: **eine** Mantelfläche je Profil-Segment.
/// Das ist die schärfere Hälfte der Herkunft — dasselbe Segment erzeugt vier
/// Kanten, von denen zwei den Nachbarn gehören, aber genau eine Fläche. Ein
/// Name, der auf Flächen aufgelöst wird, trifft deshalb, was der Aufrufer
/// meinte; derselbe Name auf Kanten nimmt die Nähte mit.
#[test]
fn test_extrude_populates_generated_faces_from_profile() {
	let profile = square(4.0);
	let profile_ids: HashSet<u64> = profile.iter().map(|e| e.id()).collect();
	let extruded = Solid::extrude(&profile, DVec3::Z * 3.0).expect("extrude");

	let gen: Vec<[u64; 2]> = extruded.iter_generated_faces().collect();
	// Genau eine Fläche je Segment — nicht vier wie bei den Kanten.
	for id in &profile_ids {
		let n = gen.iter().filter(|[_, s]| s == id).count();
		assert_eq!(n, 1, "Segment {id} wächst zu genau einer Mantelfläche, gemeldet: {n}");
	}
	let lateral: HashSet<u64> = gen.iter().filter(|[_, s]| *s != 0).map(|[f, _]| *f).collect();
	assert_eq!(lateral.len(), 4, "vier Segmente, vier verschiedene Mantelflächen");

	// Jede gemeldete Fläche gehört zum Ergebnis, und keine Ergebnis-Fläche
	// bleibt namenlos (die Deckel/Boden-Id kommt über das Kappen-Paar dazu).
	let all: HashSet<u64> = extruded.iter_face().map(|f| f.id()).collect();
	let covered: HashSet<u64> = gen.iter().map(|[f, _]| *f).collect();
	assert_eq!(all, covered, "jede Ergebnis-Fläche muss eine Herkunft haben");
}

/// **Die Messung, an der die ganze Flächen-Benennung hängt**: Deckel und Boden
/// eines Prismas teilen sich EINE TShape-Id, weil OCCT den Deckel als dieselbe
/// TShape unter anderer `TopLoc_Location` baut und `subshape_id` die Location
/// bewusst ignoriert (siehe cpp/wrapper.cpp). Ein Prisma über einem Quadrat hat
/// nach Euler 6 Flächen — gemeldet werden 5 Ids.
///
/// Folge für den Aufrufer: ein **Name** kann eine Kappe von den Mantelflächen
/// trennen, aber nicht Deckel von Boden — die Namensbindung läuft über `id()`,
/// und die muss lage-blind bleiben, damit sie eine Verschiebung übersteht.
///
/// Seit August 2026 ist das nicht mehr die einzige Auskunft: der zweite Kanal
/// `Face::key()` nimmt die Location mit und trennt die beiden
/// (`test_located_key_separates_what_the_id_merges`). Der Preis steht dort — ein
/// Schlüssel überlebt keine Bewegung. Zwei Fragen, zwei Kanäle; dieser Test
/// nagelt den lage-blinden fest, und dass er weiter auf **5** steht, ist die
/// Zusage, auf der die Namen beruhen.
#[test]
fn test_extrude_caps_share_one_tshape_id() {
	use cadrum::Surface;
	let height = 3.0;
	let extruded = Solid::extrude(&square(4.0), DVec3::Z * height).expect("extrude");

	let ids: Vec<u64> = extruded.iter_face().map(|f| f.id()).collect();
	assert_eq!(ids.len(), 6, "ein Prisma über einem Quadrat hat 6 Flächen");
	assert_eq!(ids.iter().collect::<HashSet<_>>().len(), 5, "aber nur 5 verschiedene Ids — Deckel und Boden teilen eine");

	// Die geteilte Id ist genau die, die als Kappe (Quelle 0) gemeldet wird.
	let caps: Vec<u64> = extruded.iter_generated_faces().filter(|[_, s]| *s == 0).map(|[f, _]| f).collect();
	assert_eq!(caps.len(), 1, "beide Kappen fallen auf ein Paar zusammen");
	let sharing: Vec<&cadrum::Face> = extruded.iter_face().filter(|f| f.id() == caps[0]).collect();
	assert_eq!(sharing.len(), 2, "unter der Kappen-Id liegen zwei Flächen");

	// … und die beiden sind wirklich Boden und Deckel: gleiche Ebene, gegenläufige
	// Normale, Höhe auseinander.
	let mut z: Vec<f64> = Vec::new();
	for f in &sharing {
		match f.surface() {
			Surface::Plane { origin, normal } => {
				assert!(normal.dot(DVec3::Z).abs() > 0.99, "Kappe steht senkrecht zur Zugrichtung");
				z.push(origin.z);
			}
			_ => panic!("eine Kappe ist eben"),
		}
	}
	z.sort_by(f64::total_cmp);
	assert!((z[1] - z[0] - height).abs() < 1e-9, "Boden bei 0, Deckel bei {height}, gemessen {z:?}");
}

/// revolve: bei **voller Umdrehung** meldet OCCT nur die Rotationsflächen. Die
/// Ringflächen, die aus den radialen Profil-Segmenten wachsen, kommen ohne
/// Eintrag — und Kappen gibt es keine, weil Anfang und Ende zusammenfallen.
/// Bei Teilumdrehung ist die Tabelle vollständig. Wer Flächennamen über einen
/// vollen Umlauf verspricht, verspricht zu viel; das ist die Grenze.
#[test]
fn test_revolve_full_turn_reports_only_faces_of_revolution() {
	// Rechteck 2..4 × 0..3 um Z: ein Rohrstück. Voll: 2 Zylinder + 2 Ringe.
	let profile = Edge::polygon(&[DVec3::new(2.0, 0.0, 0.0), DVec3::new(4.0, 0.0, 0.0), DVec3::new(4.0, 0.0, 3.0), DVec3::new(2.0, 0.0, 3.0)]).expect("profile");

	let full = Solid::revolve(&profile, DVec3::ZERO, DVec3::Z, TAU).expect("revolve full");
	assert_eq!(full.iter_face().count(), 4, "Rohrstück: zwei Zylinder, zwei Ringflächen");
	let gen: Vec<[u64; 2]> = full.iter_generated_faces().collect();
	assert_eq!(gen.iter().filter(|[_, s]| *s == 0).count(), 0, "voller Umlauf hat keine Kappen");
	assert_eq!(gen.len(), 2, "nur die beiden Rotationsflächen werden gemeldet");
	for [f, _] in &gen {
		let face = full.iter_face().find(|x| x.id() == *f).expect("gemeldete Fläche gehört zum Ergebnis");
		assert!(matches!(face.surface(), cadrum::Surface::Cylinder { .. }), "gemeldet werden die Zylinder");
	}
	let covered: HashSet<u64> = gen.iter().map(|[f, _]| *f).collect();
	let all: HashSet<u64> = full.iter_face().map(|f| f.id()).collect();
	assert_eq!(all.difference(&covered).count(), 2, "die beiden Ringflächen bleiben namenlos");

	// Halbe Umdrehung: jede Fläche hat eine Herkunft, und die beiden Schnittkappen
	// fallen — wie Deckel und Boden beim Prisma — auf eine Id zusammen.
	let half = Solid::revolve(&profile, DVec3::ZERO, DVec3::Z, TAU / 2.0).expect("revolve half");
	let gen_half: Vec<[u64; 2]> = half.iter_generated_faces().collect();
	let covered_half: HashSet<u64> = gen_half.iter().map(|[f, _]| *f).collect();
	let all_half: HashSet<u64> = half.iter_face().map(|f| f.id()).collect();
	assert_eq!(all_half, covered_half, "bei Teilumdrehung bleibt keine Fläche ohne Herkunft");
	assert_eq!(gen_half.iter().filter(|[_, s]| *s == 0).count(), 1, "zwei Schnittflächen, eine Id");
	assert_eq!((half.iter_face().count(), all_half.len()), (6, 5), "6 Flächen unter 5 Ids (die zwei Schnittflächen teilen eine)");
}

/// Der Weg, den ein Flächenname durch einen Boolean nimmt: die Wand einer
/// Bohrung ist `Modified(tool_face)` — sie erbt also die Identität der
/// Werkzeug-Fläche und damit deren Namen. Zusammen mit der Geburt beim extrude
/// heißt das: der Mantel eines Bohrwerkzeugs bleibt im fertigen Körper
/// auffindbar, ohne ihn geometrisch zu suchen.
#[test]
fn test_boolean_carries_a_generated_tool_face_into_the_result() {
	let circle = [Edge::circle(1.5, DVec3::Z).expect("circle")];
	let tool = Solid::extrude(&circle, DVec3::Z * 5.0).expect("tool").translate(DVec3::new(5.0, 5.0, -1.0));
	// Der Mantel des Werkzeugs, benannt aus seinem Profil (Quelle != 0).
	let wall: Vec<u64> = tool.iter_generated_faces().filter(|[_, s]| *s != 0).map(|[f, _]| f).collect();
	assert_eq!(wall.len(), 1, "ein Kreis-Profil wächst zu genau einer Mantelfläche");

	let block = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let drilled: Solid = (&block - &tool).build().expect("cut");
	let live: HashSet<u64> = drilled.iter_face().map(|f| f.id()).collect();
	let inherited: Vec<u64> = drilled.iter_history().filter(|[p, s]| *s == wall[0] && live.contains(p)).map(|[p, _]| p).collect();
	assert_eq!(inherited.len(), 1, "die Bohrungswand stammt laut history von der Werkzeugfläche");

	// Und sie ist wirklich die Bohrung: Zylinder mit dem Radius des Profils.
	let hole = drilled.iter_face().find(|f| f.id() == inherited[0]).expect("Fläche im Ergebnis");
	match hole.surface() {
		cadrum::Surface::Cylinder { radius, .. } => assert!((radius - 1.5).abs() < 1e-9, "r=1.5 erwartet, {radius}"),
		s => panic!("Bohrungswand ist ein Zylinder, nicht {s:?}"),
	}
}

/// **Ein Boolean, der zwei Flächen verschmilzt, muss BEIDE Quellen melden.**
///
/// Der Auslöser ist nicht der gestapelte Quader (dort bleiben die Seitenflächen
/// getrennt), sondern die **L-Form**: zwei Klötze mit bündigen Seitenflächen.
/// Deren koplanare Flächen werden zu je einer verschmolzen — der n→1-Fall: zwei
/// Quell-Flächen, ein Ergebnis. Gemessen, nicht angenommen; die gestapelte
/// Variante steht unten als Gegenprobe für die **Kanten**.
///
/// Die Herkunftstabelle muss dann ZWEI Paare melden, nicht eines. Meldet sie nur
/// eines, entscheidet die Hash-Reihenfolge, welche Quelle gewinnt — und ein
/// Aufrufer, der Namen an Flächen hängt (rustcads `body/naming.rs`), verliert die
/// Hälfte davon still und nicht reproduzierbar. Ein Winkel aus zwei Klötzen ist
/// die häufigste Form überhaupt, also war das kein Randfall.
#[test]
fn test_boolean_merge_reports_every_source_face() {
	let foot = Solid::cube(DVec3::ZERO, DVec3::new(20.0, 10.0, 10.0));
	let leg = Solid::cube(DVec3::ZERO, DVec3::new(10.0, 10.0, 30.0));
	let foot_faces: HashSet<u64> = foot.iter_face().map(|f| f.id()).collect();
	let leg_faces: HashSet<u64> = leg.iter_face().map(|f| f.id()).collect();
	// Ohne sechs unterscheidbare Ids je Quader misst der Rest nichts.
	assert_eq!(foot_faces.len(), 6, "ein Quader hat sechs unterscheidbare Flächen-Ids");
	assert_eq!(leg_faces.len(), 6, "ein Quader hat sechs unterscheidbare Flächen-Ids");

	let angle: Solid = (&foot + &leg).build().expect("union");
	let hist: Vec<[u64; 2]> = angle.iter_history().collect();
	let srcs: HashSet<u64> = hist.iter().map(|[_, s]| *s).collect();

	// Keine Quelle darf verschwinden: jede Fläche beider Klötze steht entweder in
	// der Tabelle oder ist gelöscht — und gelöscht wird bei dieser Union keine,
	// weil sich die beiden nur berühren und nicht durchdringen.
	let kept_foot = foot_faces.iter().filter(|f| srcs.contains(f)).count();
	let kept_leg = leg_faces.iter().filter(|f| srcs.contains(f)).count();
	assert_eq!(kept_foot, 6, "vom Fuß melden nur {kept_foot} von 6 Flächen eine Herkunft");
	assert_eq!(kept_leg, 6, "vom Schenkel melden nur {kept_leg} von 6 Flächen eine Herkunft");

	// Und die verschmolzenen Flächen nennen ZWEI verschiedene Quellen — sonst hat
	// die Tabelle den Verschmelzungsfall gar nicht abgebildet.
	let merged = posts_with_two_sources(&hist);
	assert!(merged >= 1, "keine Ergebnisfläche nennt zwei Quellen — der n→1-Kollaps ist noch da");
}

/// Dasselbe eine Dimension tiefer, und der Fall trifft **jede** gestapelte
/// Union: die Seitenflächen bleiben dort getrennt, die Kanten an der Nahtstelle
/// nicht. Ohne Multimap gingen hier vier Kanten-Namen je Stapelung verloren.
#[test]
fn test_boolean_merge_reports_every_source_edge() {
	let lower = Solid::cube(DVec3::ZERO, DVec3::new(10.0, 10.0, 10.0));
	let upper = Solid::cube(DVec3::new(0.0, 0.0, 10.0), DVec3::new(10.0, 10.0, 20.0));
	let lower_edges: HashSet<u64> = lower.iter_edge().map(|e| e.id()).collect();

	let fused: Solid = (&lower + &upper).build().expect("union");
	let hist: Vec<[u64; 2]> = fused.iter_edge_history().collect();
	let srcs: HashSet<u64> = hist.iter().map(|[_, s]| *s).collect();

	assert!(lower_edges.iter().any(|e| srcs.contains(e)), "der untere Quader ist Quelle von Ergebnis-Kanten");
	let merged = posts_with_two_sources(&hist);
	assert!(merged >= 1, "keine Ergebnis-Kante nennt zwei Quellen — der n→1-Kollaps ist noch da");
}

/// Wie viele Ergebnis-Ids nennen mindestens zwei verschiedene Quellen?
fn posts_with_two_sources(hist: &[[u64; 2]]) -> usize {
	let mut by_post: std::collections::HashMap<u64, HashSet<u64>> = std::collections::HashMap::new();
	for [p, s] in hist {
		by_post.entry(*p).or_default().insert(*s);
	}
	by_post.values().filter(|s| s.len() >= 2).count()
}

/// **Der zweite Identitätskanal — und sein Preis, in einem Test.**
///
/// `Face::key()` nimmt die `TopLoc_Location` mit (`IsSame`-Semantik), `id()`
/// wirft sie weg. Beide Zeilen dieses Tests sind die Begründung dafür, dass es
/// zwei gibt und nicht einen:
///
/// * Ein Prisma über einem Quadrat hat nach Euler sechs Flächen. `id()` meldet
///   **fünf** — Deckel und Boden sind dieselbe TShape unter zwei Locations.
///   `key()` meldet **sechs**: es trennt, was die Id verschmilzt.
/// * Nach `translate` sind die Ids **dieselben** (nur eine Location kam dazu),
///   die Schlüssel **andere**. Genau deshalb darf ein Name nie am Schlüssel
///   hängen: er stürbe bei der ersten Montage.
///
/// Ein Primitiv (Würfel) hat den Fall nicht — dort sind schon die Ids
/// verschieden. Das steht als Gegenprobe daneben, damit der Test nicht bloß
/// „key ist feiner" zeigt, sondern *wo* der Unterschied herkommt.
#[test]
fn test_located_key_separates_what_the_id_merges() {
	let prism = Solid::extrude(&square(10.0), DVec3::new(0.0, 0.0, 20.0)).expect("prism");
	assert_eq!(prism.iter_face().count(), 6, "ein Prisma über einem Quadrat hat sechs Flächen");
	let ids: HashSet<u64> = prism.iter_face().map(|f| f.id()).collect();
	let keys: HashSet<u64> = prism.iter_face().map(|f| f.key()).collect();
	assert_eq!(ids.len(), 5, "die Kappen teilen eine TShape-Id");
	assert_eq!(keys.len(), 6, "der Schlüssel trennt sie");

	// Gegenprobe: beim Primitiv gibt es nichts zu trennen.
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let cube_ids: HashSet<u64> = cube.iter_face().map(|f| f.id()).collect();
	let cube_keys: HashSet<u64> = cube.iter_face().map(|f| f.key()).collect();
	assert_eq!(cube_ids.len(), 6, "ein Würfel teilt keine TShapes");
	assert_eq!(cube_keys.len(), 6);

	// Der Preis: `share()` teilt die TShapes, `translate` setzt nur eine
	// Location — die Ids überleben das, die Schlüssel nicht.
	let moved = prism.share().translate(DVec3::new(0.0, 0.0, 5.0));
	let moved_ids: HashSet<u64> = moved.iter_face().map(|f| f.id()).collect();
	let moved_keys: HashSet<u64> = moved.iter_face().map(|f| f.key()).collect();
	assert_eq!(moved_ids, ids, "Ids überstehen eine Verschiebung — darauf beruhen die Namen");
	assert!(moved_keys.is_disjoint(&keys), "Schlüssel überstehen sie nicht, und das ist der Preis");
}

/// **Ein Prisma mit Loch, ohne Boolean** — und warum das eine Herkunftsfrage ist
/// und keine Geschwindigkeitsfrage.
///
/// `extrude_with_holes` gibt dem Kernel die Innenkonturen als innere Wires der
/// Profilfläche mit. Bis dahin wurde ein Loch nachträglich herausgeschnitten,
/// und dieser Schnitt baut die Topologie neu. Der Rebuild ist es, der die
/// Kappen-Auskunft zerreißt: Deckel und Boden teilen bei der Geburt EINE
/// TShape-Id (`test_extrude_caps_share_one_tshape_id`), und sobald ein Boolean
/// sie in zwei Ids zerlegt, sagt die Herkunftstabelle nur noch „aus X wurden A
/// und B" — welche der beiden der Deckel war, steht nirgends mehr.
///
/// Geboren mit seinem Loch behält das Prisma die Antwort: die Kappen teilen
/// weiterhin eine Id (der Schlüssel trennt sie), und die Lochwände tragen ihre
/// Herkunft direkt aus der Innenkontur statt sie von einem Werkzeug zu erben.
#[test]
fn test_extrude_with_holes_keeps_cap_provenance() {
	let outer = square(20.0);
	let hole = Edge::polygon(&[DVec3::new(5.0, 5.0, 0.0), DVec3::new(15.0, 5.0, 0.0), DVec3::new(15.0, 15.0, 0.0), DVec3::new(5.0, 15.0, 0.0)]).expect("hole polygon");
	let hole_srcs: HashSet<u64> = hole.iter().map(|e| e.id()).collect();

	let solid = Solid::extrude_with_holes([outer.iter(), hole.iter()], DVec3::new(0.0, 0.0, 4.0)).expect("extrude with hole");

	// Das Loch ist ein Loch und kein zweiter Klotz — analytisch, nicht aus der
	// Rechnung des Prüflings: (20² − 10²) · 4.
	assert!((solid.volume() - (20.0 * 20.0 - 10.0 * 10.0) * 4.0).abs() < 1e-6, "Volumen {}", solid.volume());
	assert!(solid.is_valid(), "der Kernel hält das Ergebnis für gültig");

	// Die Kappen-Auskunft steht noch: 10 Flächen, aber nur 9 Ids — Deckel und
	// Boden liegen weiter unter einer, und genau das ist die Zusage, die ein
	// nachträglicher Boolean zerstört hätte.
	assert_eq!(solid.iter_face().count(), 10, "4 außen + 4 innen + 2 Kappen");
	assert_eq!(solid.iter_face().map(|f| f.id()).collect::<HashSet<_>>().len(), 9, "die Kappen teilen eine Id");
	assert_eq!(solid.iter_face().map(|f| f.key()).collect::<HashSet<_>>().len(), 10, "der Schlüssel trennt sie");

	// Und die Lochwände sind benannt — bei der Geburt, aus ihrer eigenen
	// Innenkontur, statt von einem Schnittwerkzeug geerbt.
	let walls = solid.iter_generated_faces().filter(|[_, s]| hole_srcs.contains(s)).count();
	assert_eq!(walls, 4, "jede Innenkontur-Kante wächst zu einer Lochwand");
}

/// **Der Wicklungssinn einer Lochkontur ist nicht vorhersehbar — also wird er
/// bestimmt und nicht angenommen.**
///
/// Die erste Fassung von `extrude_with_holes` drehte jede Innenkontur pauschal
/// um. Das ist an einem Lochquadrat richtig, das im selben Sinn wie die
/// Außenkontur gezeichnet ist, und **falsch** an einem, das schon
/// entgegengesetzt läuft: die Lochfläche wird dann **addiert statt abgezogen**,
/// und OCCT meldet die Fläche trotzdem als `IsDone`. Gemessen an einer
/// Kammerkontur aus rustcads Strangprofil — 36 000 + Kammerfläche statt
/// 36 000 − Kammerfläche, ohne eine einzige Fehlermeldung.
///
/// Eine Skizze hat keinen kanonischen Umlaufsinn (er fällt aus der Zeichen-
/// reihenfolge), also darf der Bau ihn nicht raten. `ShapeFix_Face::
/// FixOrientation` entscheidet es an der Geometrie. Dieser Test hält beide
/// Fälle fest; vor dem Fix fiel genau einer von beiden durch.
#[test]
fn test_extrude_with_holes_orients_either_winding() {
	let dir = DVec3::new(0.0, 0.0, 4.0);
	let want = (20.0 * 20.0 - 10.0 * 10.0) * 4.0;

	// Loch im GLEICHEN Umlaufsinn wie die Außenkontur (beide gegen den Uhrzeiger).
	let same = Edge::polygon(&[DVec3::new(5.0, 5.0, 0.0), DVec3::new(15.0, 5.0, 0.0), DVec3::new(15.0, 15.0, 0.0), DVec3::new(5.0, 15.0, 0.0)]).expect("hole ccw");
	let a = Solid::extrude_with_holes([square(20.0).iter(), same.iter()], dir).expect("same winding");
	assert!((a.volume() - want).abs() < 1e-6, "gleicher Umlaufsinn: {} statt {want}", a.volume());
	assert!(a.is_valid());

	// Und im ENTGEGENGESETZTEN — derselbe Körper, dieselbe Zahl.
	let opposite = Edge::polygon(&[DVec3::new(5.0, 5.0, 0.0), DVec3::new(5.0, 15.0, 0.0), DVec3::new(15.0, 15.0, 0.0), DVec3::new(15.0, 5.0, 0.0)]).expect("hole cw");
	let b = Solid::extrude_with_holes([square(20.0).iter(), opposite.iter()], dir).expect("opposite winding");
	assert!((b.volume() - want).abs() < 1e-6, "entgegengesetzter Umlaufsinn: {} statt {want}", b.volume());
	assert!(b.is_valid());
}

/// **Was eine Verrundung erzeugt, sagt sie jetzt auch** — die Blend-Fläche,
/// nicht nur ihre Kanten.
///
/// `Generated(edge)` liefert die Blend-Fläche, die aus einer verrundeten Kante
/// gewachsen ist. Die Schleife hatte sie längst in der Hand und meldete nur ihre
/// **Kanten** weiter; wer fragte „welche Flächen hat mein Fillet gerade
/// gemacht?", musste dafür zurück in die Geometrie — für eine Antwort, die der
/// Builder schon gegeben hatte.
#[test]
fn test_fillet_reports_the_faces_it_generated() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let edge = cube.iter_edge().next().expect("cube has edges");
	let edge_id = edge.id();
	let before: HashSet<u64> = cube.iter_face().map(|f| f.id()).collect();

	let filleted = cube.fillet_edges(1.0, [edge]).expect("fillet");
	let gen: Vec<[u64; 2]> = filleted.iter_generated_faces().collect();
	assert_eq!(gen.len(), 1, "eine verrundete Kante wächst zu einer Blend-Fläche");

	let [face, src] = gen[0];
	assert_eq!(src, edge_id, "die Quelle ist die verrundete Kante");
	// Es ist wirklich eine Fläche des Ergebnisses …
	let after: HashSet<u64> = filleted.iter_face().map(|f| f.id()).collect();
	assert!(after.contains(&face), "die gemeldete Blend-Fläche gehört zum Ergebnis");
	// … und sie ist neu, nicht eine umbenannte alte.
	assert!(!before.contains(&face), "sie ist neu geboren, nicht Modified");
}
