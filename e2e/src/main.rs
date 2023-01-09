fn sources(local_server: Option<&str>, deployed_js_server: Option<&str>) -> Vec<String> {
	let mut vec = Vec::default();

	if let Some(local_server) = local_server {
		vec.push(local_server.to_string());
	}

	if let Some(deployed_js_server) = deployed_js_server {
		vec.push(deployed_js_server.to_string());
	}

	vec
}

enum Mode {
	StOlaf,
	Carleton,
}

fn get_substitutions(mode: &Mode, token: &str) -> Option<Vec<String>> {
	match token {
		// ":itemId" =>
		_ => None,
	}
}

fn routes(modes: Vec<Mode>) -> impl Iterator<Item = &'static str> {
	let mut vec = Vec::default();

	let stolaf_vec = vec![
		"/food/item/:itemId",
		"/food/menu/:cafeId",
		"/food/cafe/:cafeId",
		"/food/named/menu/the-pause",
		"/food/named/cafe/stav-hall",
		"/food/named/menu/stav-hall",
		"/food/named/cafe/the-cage",
		"/food/named/menu/the-cage",
		"/food/named/cafe/kings-room",
		"/food/named/menu/kings-room",
		"/food/named/cafe/burton",
		"/food/named/menu/burton",
		"/food/named/cafe/ldc",
		"/food/named/menu/ldc",
		"/food/named/cafe/sayles",
		"/food/named/menu/sayles",
		"/food/named/cafe/weitz",
		"/food/named/menu/weitz",
		"/calendar/google",
		"/calendar/reason",
		"/calendar/ics",
		"/calendar/named/stolaf",
		"/calendar/named/oleville",
		"/calendar/named/northfield",
		"/calendar/named/krlx-schedule",
		"/calendar/named/ksto-schedule",
		"/a-to-z",
		"/dictionary",
		"/directory/departments",
		"/directory/majors",
		"/contacts",
		"/tools/help",
		"/faqs",
		"/webcams",
		"/jobs",
		"/orgs",
		"/news/rss",
		"/news/wpjson",
		"/news/named/stolaf",
		"/news/named/oleville",
		"/news/named/politicole",
		"/news/named/mess",
		"/news/named/ksto",
		"/news/named/krlx",
		"/spaces/hours",
		"/transit/bus",
		"/transit/modes",
		"/streams/archived",
		"/streams/upcoming",
		"/printing/color-printers",
		"/util/html-to-md",
	];

	let carleton_vec = vec![
		"/food/item/:itemId",
		"/food/menu/:cafeId",
		"/food/cafe/:cafeId",
		"/food/named/menu/the-pause",
		"/food/named/cafe/stav-hall",
		"/food/named/menu/stav-hall",
		"/food/named/cafe/the-cage",
		"/food/named/menu/the-cage",
		"/food/named/cafe/kings-room",
		"/food/named/menu/kings-room",
		"/food/named/cafe/burton",
		"/food/named/menu/burton",
		"/food/named/cafe/ldc",
		"/food/named/menu/ldc",
		"/food/named/cafe/sayles",
		"/food/named/menu/sayles",
		"/food/named/cafe/weitz",
		"/food/named/menu/weitz",
		"/calendar/google",
		"/calendar/reason",
		"/calendar/ics",
		"/calendar/named/carleton",
		"/calendar/named/ems",
		"/calendar/named/the-cave",
		"/calendar/named/stolaf",
		"/calendar/named/northfield",
		"/calendar/named/krlx-schedule",
		"/calendar/named/ksto-schedule",
		"/calendar/named/upcoming-convos",
		"/calendar/named/sumo-schedule",
		"/dictionary",
		"/convos/upcoming",
		"/convos/upcoming/:id",
		"/convos/archived",
		"/contacts",
		"/tools/help",
		"/faqs",
		"/webcams",
		"/jobs",
		"/map",
		"/map/geojson",
		"/orgs",
		"/news/rss",
		"/news/wpjson",
		"/news/named/nnb",
		"/news/named/carleton-now",
		"/news/named/carletonian",
		"/news/named/krlx",
		"/news/named/covid",
		"/spaces/hours",
		"/transit/bus",
		"/transit/modes",
		"/util/html-to-md",
	];

	for mode in modes {
		match mode {
			Mode::StOlaf => vec.extend(&stolaf_vec),
			Mode::Carleton => vec.extend(&carleton_vec),
		}
	}

	vec.into_iter()
}

#[tokio::main]
async fn main() {
	println!("Hello, world!");
}
