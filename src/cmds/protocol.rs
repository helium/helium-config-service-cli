use super::{AddGwmpSettings, AddHttpSettings, AddPacketRouterSettings};
use crate::{
    route::Route,
    server::{Protocol, Server},
    Msg, PrettyJson, Result,
};

pub async fn add_http_protocol(args: AddHttpSettings) -> Result<Msg> {
    let http = Protocol::make_http(args.flow_type, args.dedupe_timeout, args.path);
    let server = Server::new(args.host, args.port, http);

    if !args.commit {
        return Msg::ok(format!("valid http settings\n{}", server.pretty_json()?));
    }

    let mut route = Route::from_file(&args.route_file)?;
    route.set_server(server);
    route.write(&args.route_file)?;

    Msg::ok(format!("{} written", args.route_file.display()))
}

pub async fn add_gwmp_protocol(args: AddGwmpSettings) -> Result<Msg> {
    let gwmp = match (args.region, args.region_port) {
        (Some(region), Some(region_port)) => Protocol::make_gwmp(region, region_port)?,
        (None, None) => Protocol::default_gwmp(),
        _ => return Msg::err("Must provide both `region` and `region_port`".to_string()),
    };
    let server = Server::new(args.host, args.port, gwmp);

    if !args.commit {
        return Msg::ok(format!("valid gwmp settings\n{}", server.pretty_json()?));
    }

    let mut route = Route::from_file(&args.route_file)?;
    route.set_server(server);
    route.write(&args.route_file)?;

    Msg::ok(
        [
            format!("{} written", args.route_file.display()),
            "To add more region mapping, use the command `add gwmp-mapping`".to_string(),
        ]
        .join("\n"),
    )
}

pub async fn add_packet_router_protocol(args: AddPacketRouterSettings) -> Result<Msg> {
    let packet_router = Protocol::default_packet_router();
    let server = Server::new(args.host, args.port, packet_router);

    if !args.commit {
        return Msg::ok(format!(
            "valid packet router settings\n{}",
            server.pretty_json()?,
        ));
    }

    let mut route = Route::from_file(&args.route_file)?;
    route.set_server(server);
    route.write(&args.route_file)?;

    Msg::ok(format!("{} written", args.route_file.display()))
}
