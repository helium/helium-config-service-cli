use crate::{
    client,
    cmds::{
        CreateRoute, GenerateRoute, GetRoute, GetRoutes, PathBufKeypair, RemoveRoute, SubnetMask,
        UpdateRoute,
    },
    route::Route,
    server::Protocol,
    subnet::RouteSubnets,
    DevaddrConstraint, Msg, PrettyJson, Result,
};

use super::{AddDevaddr, AddEui, AddGwmpMapping};

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
    let route_list = client
        .list(args.oui, &args.owner, &args.keypair.to_keypair()?)
        .await?;

    if args.commit {
        route_list.write_all(&args.route_out_dir)?;
        return Msg::ok(format!("{} routes written", route_list.count()));
    }

    Msg::ok(route_list.pretty_json()?)
}

pub async fn get_route(args: GetRoute) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host).await?;
    let route = client
        .get(&args.route_id, &args.owner, &args.keypair.to_keypair()?)
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
            .create_route(route, &args.owner, &args.keypair.to_keypair()?)
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

pub async fn update_route(args: UpdateRoute) -> Result<Msg> {
    let route = Route::from_file(&args.route_file)?;
    if args.commit {
        let mut client = client::RouteClient::new(&args.config_host).await?;
        let updated_route = client
            .push(route, &args.owner, &args.keypair.to_keypair()?)
            .await?;
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
            .delete(&route.id, &args.owner, &args.keypair.to_keypair()?)
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
