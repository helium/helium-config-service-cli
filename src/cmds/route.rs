use super::{
    ActivateRoute, AddGwmpRegion, DeactivateRoute, DeleteRoute, GetRoute, ListRoutes, NewRoute,
    RemoveGwmpRegion, UpdateHttp, UpdateMaxCopies, UpdatePacketRouter, UpdateServer,
};
use crate::{
    client, cmds::PathBufKeypair, route::Route, server::Protocol, Msg, PrettyJson, Result,
};

pub async fn list_routes(args: ListRoutes) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host, &args.config_pubkey).await?;
    match client.list(args.oui, &args.keypair.to_keypair()?).await {
        Ok(route_list) => Msg::ok(route_list.pretty_json()?),
        Err(err) => Msg::err(format!("could not list routes: {err}")),
    }
}

pub async fn get_route(args: GetRoute) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host, &args.config_pubkey).await?;
    match client
        .get(&args.route_id, &args.keypair.to_keypair()?)
        .await
    {
        Ok(route) => Msg::ok(route.pretty_json()?),
        Err(err) => Msg::err(format!("could not get route: {err}")),
    }
}

pub async fn new_route(args: NewRoute) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host, &args.config_pubkey).await?;
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
    let mut client = client::RouteClient::new(&args.config_host, &args.config_pubkey).await?;

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
    let mut client = client::RouteClient::new(&args.config_host, &args.config_pubkey).await?;
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
    let mut client = client::RouteClient::new(&args.config_host, &args.config_pubkey).await?;
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
    let mut client = client::RouteClient::new(&args.config_host, &args.config_pubkey).await?;
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
    let mut client = client::RouteClient::new(&args.config_host, &args.config_pubkey).await?;
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
    let mut client = client::RouteClient::new(&args.config_host, &args.config_pubkey).await?;
    let keypair = args.keypair.to_keypair()?;

    let mut route = client.get(&args.route_id, &keypair).await?;
    let old_route = route.clone();

    let old_protocol = route.server.protocol;

    let mut new_protocol = if let Some(p) = old_protocol.as_ref() {
        p.clone()
    } else {
        return Msg::err("Cannot remove region mapping, no protocol".to_string());
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
    let mut client = client::RouteClient::new(&args.config_host, &args.config_pubkey).await?;
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

pub async fn activate_route(args: ActivateRoute) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host, &args.config_pubkey).await?;
    let keypair = args.keypair.to_keypair()?;

    let mut route = client.get(&args.route_id, &keypair).await?;
    let old_route = route.clone();

    route.active = true;

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
        Err(err) => Msg::err(format!("Could not activate route: {err}")),
    }
}

pub async fn deactivate_route(args: DeactivateRoute) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host, &args.config_pubkey).await?;
    let keypair = args.keypair.to_keypair()?;

    let mut route = client.get(&args.route_id, &keypair).await?;
    let old_route = route.clone();

    route.active = false;

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
        Err(err) => Msg::err(format!("Could not deactivate route: {err}")),
    }
}

pub mod skfs {
    use crate::{
        client,
        cmds::{AddFilter, GetFilters, ListFilters, PathBufKeypair, RemoveFilter, UpdateFilters},
        Msg, PrettyJson, Result, Skf, SkfUpdate,
    };
    use anyhow::Context;

    pub async fn list_filters(args: ListFilters) -> Result<Msg> {
        let mut client = client::SkfClient::new(&args.config_host, &args.config_pubkey).await?;
        let filters = client
            .list_filters(&args.route_id, &args.keypair.to_keypair()?)
            .await?;

        Msg::ok(filters.pretty_json()?)
    }

    pub async fn get_filters(args: GetFilters) -> Result<Msg> {
        let mut client = client::SkfClient::new(&args.config_host, &args.config_pubkey).await?;
        let filters = client
            .get_filters(&args.route_id, args.devaddr, &args.keypair.to_keypair()?)
            .await?;

        Msg::ok(filters.pretty_json()?)
    }

    pub async fn add_filter(args: AddFilter) -> Result<Msg> {
        let mut client = client::SkfClient::new(&args.config_host, &args.config_pubkey).await?;
        let filter = Skf::new(args.route_id.clone(), args.devaddr, args.session_key)?;

        if !args.commit {
            return Msg::dry_run(format!("added {filter:?}"));
        }

        client
            .add_filter(filter.clone(), &args.keypair.to_keypair()?)
            .await?;

        Msg::ok(format!("added {filter:?}"))
    }

    pub async fn remove_filter(args: RemoveFilter) -> Result<Msg> {
        let mut client = client::SkfClient::new(&args.config_host, &args.config_pubkey).await?;
        let filter = Skf::new(args.route_id.clone(), args.devaddr, args.session_key)?;

        if !args.commit {
            return Msg::dry_run(format!("removed {filter:?}"));
        }

        client
            .remove_filter(filter.clone(), &args.keypair.to_keypair()?)
            .await?;

        Msg::ok(format!("removed {filter:?}"))
    }

    pub async fn update_filters_from_file(args: UpdateFilters) -> Result<Msg> {
        let mut client = client::SkfClient::new(&args.config_host, &args.config_pubkey).await?;

        let data = std::fs::read_to_string(&args.update_file)
            .context("reading session key filter updates json file")?;
        let updates: Vec<SkfUpdate> = serde_json::from_str(&data).context(format!(
            "parsing session key filter update file {}",
            &args.update_file.display()
        ))?;

        let update_count = updates.len();
        if update_count > 100 {
            return Msg::err("exceeds max 100 update limit per request".to_string());
        }

        if !args.commit {
            return Msg::dry_run(format!("updated filters applied {update_count}"));
        }

        client
            .update_filters(&args.route_id, updates, &args.keypair.to_keypair()?)
            .await?;

        Msg::ok("updated filters".to_string())
    }
}

pub mod euis {
    use crate::{
        client,
        cmds::{AddEui, ClearEuis, ListEuis, PathBufKeypair, RemoveEui},
        Eui, Msg, PrettyJson, Result,
    };

    pub async fn list_euis(args: ListEuis) -> Result<Msg> {
        let mut client = client::EuiClient::new(&args.config_host, &args.config_pubkey).await?;
        let euis_for_route = client
            .get_euis(&args.route_id, &args.keypair.to_keypair()?)
            .await?;

        Msg::ok(euis_for_route.pretty_json()?)
    }

    pub async fn add_eui(args: AddEui) -> Result<Msg> {
        let mut client = client::EuiClient::new(&args.config_host, &args.config_pubkey).await?;
        let eui_pair = Eui::new(args.route_id.clone(), args.app_eui, args.dev_eui)?;

        if !args.commit {
            return Msg::dry_run(format!("added {eui_pair:?} to {}", args.route_id));
        }

        client
            .add_euis(vec![eui_pair.clone()], &args.keypair.to_keypair()?)
            .await?;

        Msg::ok(format!("added {eui_pair:?} to {}", args.route_id))
    }

    pub async fn remove_eui(args: RemoveEui) -> Result<Msg> {
        let mut client = client::EuiClient::new(&args.config_host, &args.config_pubkey).await?;
        let eui_pair = Eui::new(args.route_id.clone(), args.app_eui, args.dev_eui)?;

        if !args.commit {
            return Msg::dry_run(format!("removed {eui_pair:?} from {}", args.route_id));
        }

        client
            .remove_euis(vec![eui_pair.clone()], &args.keypair.to_keypair()?)
            .await?;

        Msg::ok(format!("removed {eui_pair:?} from {}", args.route_id))
    }

    pub async fn clear_euis(args: ClearEuis) -> Result<Msg> {
        let mut client = client::EuiClient::new(&args.config_host, &args.config_pubkey).await?;

        if !args.commit {
            return Msg::dry_run(format!("All Euis removed from {}", args.route_id));
        }

        client
            .delete_euis(args.route_id.clone(), &args.keypair.to_keypair()?)
            .await?;
        Msg::ok(format!("All Euis removed from {}", args.route_id))
    }
}

pub mod devaddrs {
    use crate::{
        client,
        cmds::{
            AddDevaddr, ClearDevaddrs, ListDevaddrs, PathBufKeypair, RemoveDevaddr, RouteSubnetMask,
        },
        subnet::DevaddrSubnet,
        DevaddrRange, Msg, PrettyJson, Result,
    };

    pub async fn list_devaddrs(args: ListDevaddrs) -> Result<Msg> {
        let mut client = client::DevaddrClient::new(&args.config_host, &args.config_pubkey).await?;
        let devaddrs_for_route = client
            .get_devaddrs(&args.route_id, &args.keypair.to_keypair()?)
            .await?;

        Msg::ok(devaddrs_for_route.pretty_json()?)
    }

    pub async fn add_devaddr(args: AddDevaddr) -> Result<Msg> {
        let mut client = client::DevaddrClient::new(&args.config_host, &args.config_pubkey).await?;
        let devaddr_range =
            DevaddrRange::new(args.route_id.clone(), args.start_addr, args.end_addr)?;

        if !args.commit {
            return Msg::dry_run(format!("added {devaddr_range:?}"));
        }

        client
            .add_devaddrs(vec![devaddr_range.clone()], &args.keypair.to_keypair()?)
            .await?;

        Msg::ok(format!("added {devaddr_range:?}"))
    }

    pub async fn remove_devaddr(args: RemoveDevaddr) -> Result<Msg> {
        let mut client = client::DevaddrClient::new(&args.config_host, &args.config_pubkey).await?;
        let devaddr_range =
            DevaddrRange::new(args.route_id.clone(), args.start_addr, args.end_addr)?;

        if !args.commit {
            return Msg::dry_run(format!("removed {devaddr_range:?} from {}", args.route_id));
        }

        client
            .remove_devaddrs(vec![devaddr_range.clone()], &args.keypair.to_keypair()?)
            .await?;

        Msg::ok(format!("removed {devaddr_range:?} from {}", args.route_id))
    }

    pub async fn clear_devaddrs(args: ClearDevaddrs) -> Result<Msg> {
        let mut client = client::DevaddrClient::new(&args.config_host, &args.config_pubkey).await?;

        if !args.commit {
            return Msg::dry_run(format!("All Devadddrs removed from {}", args.route_id));
        }

        client
            .delete_devaddrs(args.route_id.clone(), &args.keypair.to_keypair()?)
            .await?;

        Msg::ok(format!("All Devaddrs removed from {}", args.route_id))
    }

    pub async fn subnet_mask(args: RouteSubnetMask) -> Result<Msg> {
        let mut client = client::DevaddrClient::new(&args.config_host, &args.config_pubkey).await?;
        let devaddrs_for_route: Vec<DevaddrSubnet> = client
            .get_devaddrs(&args.route_id, &args.keypair.to_keypair()?)
            .await?
            .into_iter()
            .map(|range| range.to_subnet())
            .collect();
        Msg::ok(devaddrs_for_route.pretty_json()?)
    }
}
