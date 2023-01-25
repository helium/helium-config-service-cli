use crate::{
    client,
    cmds::{
        CreateRoute, GenerateRoute, GetRouteOld, GetRoutes, PathBufKeypair, RemoveRoute,
        SubnetMask, UpdateRouteOld,
    },
    route::Route,
    server::Protocol,
    subnet::RouteSubnets,
    DevaddrConstraint, Msg, PrettyJson, Result,
};

use super::{
    AddDevaddr, AddEui, AddGwmpMapping, AddGwmpRegion, DeleteRoute, GetRoute, ListRoutes, NewRoute,
    RemoveGwmpRegion, UpdateHttp, UpdateMaxCopies, UpdatePacketRouter, UpdateServer,
};

pub async fn list_routes(args: ListRoutes) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host).await?;
    match client.list(args.oui, &args.keypair.to_keypair()?).await {
        Ok(route_list) => Msg::ok(route_list.pretty_json()?),
        Err(err) => Msg::err(format!("could not list routes: {err}")),
    }
}

pub async fn get_route(args: GetRoute) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host).await?;
    match client
        .get(&args.route_id, &args.keypair.to_keypair()?)
        .await
    {
        Ok(route) => Msg::ok(route.pretty_json()?),
        Err(err) => Msg::err(format!("could not get route: {err}")),
    }
}

pub async fn new_route(args: NewRoute) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host).await?;
    let route = Route::new(args.net_id, args.oui, args.max_copies);

    if !args.commit {
        return Msg::dry_run(route.pretty_json()?);
    }

    match client
        .create_route(route, &args.keypair.to_keypair()?)
        .await
    {
        Ok(created_route) => Msg::ok(format!(
            "created route {}\n{}",
            created_route.id,
            created_route.pretty_json()?
        )),
        Err(err) => Msg::err(format!("route not created: {err}")),
    }
}

pub async fn delete_route(args: DeleteRoute) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host).await?;

    if !args.commit {
        return Msg::dry_run(format!("delete {}", args.route_id));
    }

    match client
        .delete(&args.route_id, &args.keypair.to_keypair()?)
        .await
    {
        Ok(removed_route) => Msg::ok(format!("deleted route {}", removed_route.id)),
        Err(err) => Msg::err(format!("route not deleted: {err}")),
    }
}

pub async fn update_max_copies(args: UpdateMaxCopies) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host).await?;
    let keypair = args.keypair.to_keypair()?;

    let mut route = client.get(&args.route_id, &keypair).await?;
    let old_route = route.clone();

    route.max_copies = args.max_copies;

    if !args.commit {
        return Msg::dry_run(format!(
            "Updated {}\n== Old\n{}\n== New\n{}",
            route.id,
            old_route.pretty_json()?,
            route.pretty_json()?
        ));
    }

    match client.push(route, &keypair).await {
        Ok(updated_route) => Msg::ok(format!(
            "Updated {}\n== Old\n{}\n== New\n{}",
            updated_route.id,
            old_route.pretty_json()?,
            updated_route.pretty_json()?
        )),
        Err(err) => Msg::err(format!("could not update max_copies: {err}")),
    }
}

pub async fn update_server(args: UpdateServer) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host).await?;
    let keypair = args.keypair.to_keypair()?;

    let mut route = client.get(&args.route_id, &keypair).await?;
    let old_route = route.clone();

    route.server.host = args.host;
    route.server.port = args.port;

    if !args.commit {
        return Msg::dry_run(format!(
            "Updated {}\n== Old\n{}\n== New\n{}",
            route.id,
            old_route.pretty_json()?,
            route.pretty_json()?
        ));
    }

    match client.push(route, &keypair).await {
        Ok(updated_route) => Msg::ok(format!(
            "Updated {}\n== Old\n{}\n== New\n{}",
            updated_route.id,
            old_route.pretty_json()?,
            updated_route.pretty_json()?
        )),

        Err(err) => Msg::err(format!("could not update server host and port: {err}")),
    }
}

pub async fn update_http(args: UpdateHttp) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host).await?;
    let keypair = args.keypair.to_keypair()?;

    let mut route = client.get(&args.route_id, &keypair).await?;
    let old_route = route.clone();

    let http = Protocol::make_http(args.dedupe_timeout, args.path, args.auth_header);
    route.server.protocol = Some(http);

    if !args.commit {
        return Msg::dry_run(format!(
            "Updated {}\n== Old\n{}\n== New\n{}",
            route.id,
            old_route.pretty_json()?,
            route.pretty_json()?
        ));
    }

    match client.push(route, &keypair).await {
        Ok(updated_route) => Msg::ok(format!(
            "Updated {}\n== Old\n{}\n== New\n{}",
            updated_route.id,
            old_route.pretty_json()?,
            updated_route.pretty_json()?
        )),
        Err(err) => Msg::err(format!("Could not update http protocol: {err}")),
    }
}

pub async fn add_gwmp_region(args: AddGwmpRegion) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host).await?;
    let keypair = args.keypair.to_keypair()?;

    let mut route = client.get(&args.route_id, &keypair).await?;
    let old_route = route.clone();
    let old_protocol = route.server.protocol;

    let gwmp = if let Some(protocol) = old_protocol.as_ref() {
        if protocol.is_gwmp() {
            let mut new_protocol = protocol.clone();
            let map = Protocol::make_gwmp_mapping(args.region, args.region_port);
            new_protocol.gwmp_add_mapping(map)?;
            new_protocol
        } else {
            Protocol::make_gwmp(args.region, args.region_port)?
        }
    } else {
        Protocol::make_gwmp(args.region, args.region_port)?
    };

    route.server.protocol = Some(gwmp);

    if !args.commit {
        return Msg::dry_run(format!(
            "Updated {}\n== Old\n{}\n== New\n{}",
            route.id,
            old_route.pretty_json()?,
            route.pretty_json()?
        ));
    }

    match client.push(route, &keypair).await {
        Ok(updated_route) => Msg::ok(format!(
            "Updated {}\n== Old\n{}\n== New\n{}",
            updated_route.id,
            old_route.pretty_json()?,
            updated_route.pretty_json()?
        )),
        Err(err) => Msg::err(format!("Could not update gwmp protocol: {err}")),
    }
}

pub async fn remove_gwmp_region(args: RemoveGwmpRegion) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host).await?;
    let keypair = args.keypair.to_keypair()?;

    let mut route = client.get(&args.route_id, &keypair).await?;
    let old_route = route.clone();

    let old_protocol = route.server.protocol;

    let mut new_protocol = if let Some(p) = old_protocol.as_ref() {
        p.clone()
    } else {
        return Msg::err(format!("Cannot remove region mapping, no protocol"));
    };
    new_protocol.gwmp_remove_mapping(&args.region)?;

    route.server.protocol = Some(new_protocol);

    if !args.commit {
        return Msg::dry_run(format!(
            "Updated {}\n== Old\n{}\n== New\n{}",
            route.id,
            old_route.pretty_json()?,
            route.pretty_json()?
        ));
    }

    match client.push(route, &keypair).await {
        Ok(updated_route) => Msg::ok(format!(
            "Updated {}\n== Old\n{}\n== New\n{}",
            updated_route.id,
            old_route.pretty_json()?,
            updated_route.pretty_json()?
        )),
        Err(err) => Msg::err(format!("Could not update gwmp protocol: {err}")),
    }
}

pub async fn update_packet_router(args: UpdatePacketRouter) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host).await?;
    let keypair = args.keypair.to_keypair()?;

    let mut route = client.get(&args.route_id, &keypair).await?;
    let old_route = route.clone();

    let new_protocol = Protocol::default_packet_router();
    route.server.protocol = Some(new_protocol);

    if !args.commit {
        return Msg::dry_run(format!(
            "Updated {}\n== Old\n{}\n== New\n{}",
            route.id,
            old_route.pretty_json()?,
            route.pretty_json()?
        ));
    }

    match client.push(route, &keypair).await {
        Ok(updated_route) => Msg::ok(format!(
            "Updated {}\n== Old\n{}\n== New\n{}",
            updated_route.id,
            old_route.pretty_json()?,
            updated_route.pretty_json()?
        )),
        Err(_) => todo!(),
    }
}

pub mod euis {
    use crate::{
        client,
        cmds::{AddEuis, DeleteEuis, GetEuis, PathBufKeypair, RemoveEuis},
        Eui, Msg, PrettyJson, Result,
    };

    pub async fn get_euis(args: GetEuis) -> Result<Msg> {
        let mut client = client::EuiClient::new(&args.config_host).await?;
        let euis_for_route = client
            .get_euis(&args.route_id, &args.keypair.to_keypair()?)
            .await?;

        Msg::ok(euis_for_route.pretty_json()?)
    }

    pub async fn add_euis(args: AddEuis) -> Result<Msg> {
        let mut client = client::EuiClient::new(&args.config_host).await?;
        let eui_pair = Eui::new(args.app_eui, args.dev_eui)?;

        client
            .add_euis(
                args.route_id.clone(),
                vec![eui_pair.clone()],
                &args.keypair.to_keypair()?,
            )
            .await?;

        Msg::ok(format!("added {eui_pair:?} to {}", args.route_id))
    }

    pub async fn remove_euis(args: RemoveEuis) -> Result<Msg> {
        let mut client = client::EuiClient::new(&args.config_host).await?;
        let eui_pair = Eui::new(args.app_eui, args.dev_eui)?;

        client
            .remove_euis(
                args.route_id.clone(),
                vec![eui_pair.clone()],
                &args.keypair.to_keypair()?,
            )
            .await?;

        Msg::ok(format!("removed {eui_pair:?} from {}", args.route_id))
    }

    pub async fn delete_euis(args: DeleteEuis) -> Result<Msg> {
        let mut client = client::EuiClient::new(&args.config_host).await?;
        client
            .delete_euis(args.route_id.clone(), &args.keypair.to_keypair()?)
            .await?;
        Msg::ok(format!("All Euis removed from {}", args.route_id))
    }
}

pub mod devaddrs {
    use crate::{
        client,
        cmds::{AddDevaddrs, DeleteDevaddrs, GetDevaddrs, PathBufKeypair, RemoveDevaddrs},
        DevaddrRange, Msg, PrettyJson, Result,
    };

    pub async fn get_devaddrs(args: GetDevaddrs) -> Result<Msg> {
        let mut client = client::DevaddrClient::new(&args.config_host).await?;
        let devaddrs_for_route = client
            .get_devaddrs(&args.route_id, &args.keypair.to_keypair()?)
            .await?;

        Msg::ok(devaddrs_for_route.pretty_json()?)
    }

    pub async fn add_devaddrs(args: AddDevaddrs) -> Result<Msg> {
        let mut client = client::DevaddrClient::new(&args.config_host).await?;
        let devaddr_range =
            DevaddrRange::new(args.route_id.clone(), args.start_addr, args.end_addr)?;

        client
            .add_devaddrs(
                args.route_id,
                vec![devaddr_range.clone()],
                &args.keypair.to_keypair()?,
            )
            .await?;

        Msg::ok(format!("added {devaddr_range:?}"))
    }

    pub async fn remove_devaddrs(args: RemoveDevaddrs) -> Result<Msg> {
        let mut client = client::DevaddrClient::new(&args.config_host).await?;
        let devaddr_range =
            DevaddrRange::new(args.route_id.clone(), args.start_addr, args.end_addr)?;

        client
            .remove_devaddrs(
                args.route_id,
                vec![devaddr_range.clone()],
                &args.keypair.to_keypair()?,
            )
            .await?;

        Msg::ok(format!("removed {devaddr_range:?}"))
    }

    pub async fn delete_devaddrs(args: DeleteDevaddrs) -> Result<Msg> {
        let mut client = client::DevaddrClient::new(&args.config_host).await?;

        client
            .delete_devaddrs(args.route_id.clone(), &args.keypair.to_keypair()?)
            .await?;

        Msg::ok(format!("All Devaddrs removed from {}", args.route_id))
    }
}

pub fn generate_route(args: GenerateRoute) -> Result<Msg> {
    if args.out_file.exists() && !args.commit {
        return Msg::err(format!(
            "{} exists, pass `--commit` to override",
            args.out_file.display()
        ));
    }

    let route = Route::new(args.net_id, args.oui, args.max_copies);
    route.write(&args.out_file)?;

    Msg::ok(format!("{} created", args.out_file.display()))
}

pub async fn get_routes(args: GetRoutes) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host).await?;
    let route_list = client.list(args.oui, &args.keypair.to_keypair()?).await?;

    if args.commit {
        route_list.write_all(&args.route_out_dir)?;
        return Msg::ok(format!("{} routes written", route_list.count()));
    }

    Msg::ok(route_list.pretty_json()?)
}

pub async fn get_route_old(args: GetRouteOld) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host).await?;
    let route = client
        .get(&args.route_id, &args.keypair.to_keypair()?)
        .await?;

    if args.commit {
        route.write(&args.route_out_dir)?;
        return Msg::ok(format!(
            "{}/{} written",
            &args.route_out_dir.display(),
            route.filename()
        ));
    }
    Msg::ok(route.pretty_json()?)
}

pub async fn create_route(args: CreateRoute) -> Result<Msg> {
    let route = Route::from_file(&args.route_file)?;

    if !route.id.is_empty() {
        return Msg::err("Route already has an ID, cannot be created".to_string());
    }

    if args.commit {
        let mut client = client::RouteClient::new(&args.config_host).await?;
        match client
            .create_route(route, &args.keypair.to_keypair()?)
            .await
        {
            Ok(created_route) => {
                // Write to both locations to prevent re-creation of route after
                // ID is assigned.
                created_route.write(&args.route_out_dir)?;
                created_route.write(&args.route_file)?;

                return Msg::ok(format!(
                    "{}/{} written",
                    &args.route_out_dir.display(),
                    created_route.filename()
                ));
            }
            Err(err) => {
                // TODO: print this prettier
                return Msg::err(format!("route not created: {err}"));
            }
        }
    }
    Msg::ok(format!(
        "{} is valid, pass `--commit` to create",
        &args.route_file.display()
    ))
}

pub async fn update_route(args: UpdateRouteOld) -> Result<Msg> {
    let route = Route::from_file(&args.route_file)?;
    if args.commit {
        let mut client = client::RouteClient::new(&args.config_host).await?;
        let updated_route = client.push(route, &args.keypair.to_keypair()?).await?;
        updated_route.write(args.route_file.as_path())?;
        return Msg::ok(format!("{} written", &args.route_file.display()));
    }
    Msg::ok(format!(
        "{} is valid, pass `--commit` to update",
        &args.route_file.display()
    ))
}

pub async fn remove_route(args: RemoveRoute) -> Result<Msg> {
    let route = Route::from_file(&args.route_file)?;
    if args.commit {
        let mut client = client::RouteClient::new(&args.config_host).await?;
        let removed_route = client
            .delete(&route.id, &args.keypair.to_keypair()?)
            .await?;
        removed_route.remove(
            args.route_file
                .parent()
                .expect("filename is in a directory"),
        )?;
        return Msg::ok(format!("{} deleted", &args.route_file.display()));
    }
    Msg::ok(format!(
        "{} ready for deletion, pass `--commit` to remove",
        &args.route_file.display()
    ))
}

pub fn subnet_mask(args: SubnetMask) -> Result<Msg> {
    if let (Some(start), Some(end)) = (args.start_addr, args.end_addr) {
        let devaddr_range = DevaddrConstraint::new(start, end)?;
        return Msg::ok(devaddr_range.to_subnet().pretty_json()?);
    }

    if let Some(path) = args.route_file {
        let routes = if path.is_file() {
            vec![Route::from_file(&path)?]
        } else {
            Route::from_dir(&path)?
        };

        let mut output = vec![];
        for route in routes {
            output.push(RouteSubnets::from_route(route))
        }
        return Msg::ok(output.pretty_json()?);
    }

    Msg::err("not enough arguments, run again with `--help`".to_string())
}

pub async fn add_devaddr(_args: AddDevaddr) -> Result<Msg> {
    unimplemented!("adding devaddr range to route");
    // let devaddr = DevaddrRange::new(args.start_addr, args.end_addr)?;
    // if !args.commit {
    //     return Msg::ok(format!(
    //         "valid range, insert into `devaddr_ranges` section\n{}",
    //         devaddr.pretty_json()?
    //     ));
    // }

    // let mut route = Route::from_file(&args.route_file)?;
    // route.add_devaddr(devaddr);
    // route.write(&args.route_file)?;
    // Msg::ok(format!("{} written", args.route_file.display()))
}

pub async fn add_eui(_args: AddEui) -> Result<Msg> {
    unimplemented!("adding EUI to route");
    // let eui = Eui::new(args.app_eui, args.dev_eui)?;
    // if !args.commit {
    //     return Msg::ok(format!(
    //         "valid eui, insert into `euis` section\n{}",
    //         eui.pretty_json()?
    //     ));
    // }

    // let mut route = Route::from_file(&args.route_file)?;
    // route.add_eui(eui);
    // route.write(&args.route_file)?;
    // Msg::ok(format!("{} written", args.route_file.display()))
}

pub async fn add_gwmp_mapping(args: AddGwmpMapping) -> Result<Msg> {
    let mapping = Protocol::make_gwmp_mapping(args.region, args.port);

    if !args.commit {
        return Msg::ok(format!(
            "valid mapping, insert into `mapping` section\n{}",
            mapping.pretty_json()?
        ));
    }

    let mut route = Route::from_file(&args.route_file)?;
    route.gwmp_add_mapping(mapping)?;
    route.write(&args.route_file)?;
    Msg::ok(format!("{} written", args.route_file.display()))
}
