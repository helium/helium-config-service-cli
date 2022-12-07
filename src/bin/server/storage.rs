use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};

use anyhow::anyhow;
use helium_config_service_cli::{
    hex_field::{self, HexDevAddr},
    proto::OrgV1,
    route::Route,
    DevaddrRange, Org, Result,
};
use helium_crypto::PublicKey;
use helium_proto::services::config::{ActionV1, SessionKeyFilterV1};
use helium_proto::services::config::{RouteStreamResV1, SessionKeyFilterStreamResV1};
use tokio::sync::broadcast::{Receiver, Sender};
use tracing::info;

pub type OUI = u64;
pub type OrgMap = RwLock<HashMap<OUI, DbOrg>>;
pub type RouteMap = RwLock<HashMap<OUI, Vec<Route>>>;
pub type FilterMap = RwLock<HashMap<OUI, SessionKeyFilter>>;

#[derive(Debug)]
pub struct Storage {
    orgs: OrgMap,
    routes: RouteMap,
    filters: FilterMap,
    next_oui: Mutex<u64>,
    next_helium_devaddr: Mutex<HexDevAddr>,
    route_update_channel: Arc<Sender<RouteStreamResV1>>,
    filter_update_channel: Arc<Sender<SessionKeyFilterStreamResV1>>,
}

pub trait OrgStorage {
    fn next_oui(&self) -> u64;
    fn create_helium_org(&self, org: Org, devaddr_constraints: DevaddrRange);
    fn create_roamer_org(&self, org: Org, devaddr_constraints: DevaddrRange);
    fn get_orgs(&self) -> Vec<DbOrg>;
    fn get_org(&self, oui: u64) -> Option<DbOrg>;
}

pub trait RouteStorage {
    fn get_routes(&self, oui: u64) -> Result<Vec<Route>>;
    fn get_route(&self, route_id: String) -> Option<Route>;
    fn create_route(&self, oui: u64, route: Route) -> Result<Route>;
    fn update_route(&self, route: Route) -> Result<Route>;
    fn delete_route(&self, route_id: String) -> Option<Route>;
    fn subscribe_to_routes(&self) -> Receiver<RouteStreamResV1>;
}

pub trait SkfStorage {
    fn get_filters(&self, oui: OUI) -> Result<Vec<SessionKeyFilter>>;
    fn get_filter(&self, oui: OUI) -> Result<SessionKeyFilter>;
    fn create_filter(&self, oui: OUI, filter: SessionKeyFilter) -> Result<SessionKeyFilter>;
    fn update_filter(&self, oui: OUI, filter: SessionKeyFilter) -> Result<SessionKeyFilter>;
    fn delete_filter(&self, oui: OUI) -> Result<SessionKeyFilter>;
    fn subscribe_to_filters(&self) -> Receiver<SessionKeyFilterStreamResV1>;
}

#[derive(Debug, Clone)]
pub struct SessionKeyFilter {
    devaddr: HexDevAddr,
    session_keys: Vec<PublicKey>,
}

impl From<SessionKeyFilterV1> for SessionKeyFilter {
    fn from(filter: SessionKeyFilterV1) -> Self {
        Self {
            devaddr: (filter.devaddr as u64).into(),
            session_keys: filter
                .session_keys
                .into_iter()
                .map(|sk| PublicKey::try_from(sk))
                .flatten()
                .collect(),
        }
    }
}

impl From<SessionKeyFilter> for SessionKeyFilterV1 {
    fn from(filter: SessionKeyFilter) -> Self {
        Self {
            devaddr: filter.devaddr.0 as i64,
            session_keys: filter
                .session_keys
                .into_iter()
                .map(|pk| pk.into())
                .collect(),
        }
    }
}

impl SkfStorage for Storage {
    fn get_filters(&self, _oui: OUI) -> Result<Vec<SessionKeyFilter>> {
        Ok(self
            .filters
            .read()
            .expect("filter store lock")
            .clone()
            .into_values()
            .collect())
    }

    fn get_filter(&self, oui: OUI) -> Result<SessionKeyFilter> {
        self.filters
            .read()
            .expect("filter store lock")
            .get(&oui)
            .clone()
            .map(|x| x.to_owned())
            .ok_or(anyhow!("filter not found"))
    }

    fn create_filter(&self, oui: OUI, filter: SessionKeyFilter) -> Result<SessionKeyFilter> {
        self.filters
            .write()
            .expect("filter write lock")
            .insert(oui, filter.clone());
        Ok(filter)
    }

    fn update_filter(&self, oui: OUI, filter: SessionKeyFilter) -> Result<SessionKeyFilter> {
        self.filters
            .write()
            .expect("filter write lock")
            .insert(oui, filter.clone());
        Ok(filter)
    }

    fn delete_filter(&self, oui: OUI) -> Result<SessionKeyFilter> {
        self.filters
            .write()
            .expect("filter write lock")
            .remove(&oui)
            .ok_or(anyhow!("could not delete filter"))
    }

    fn subscribe_to_filters(&self) -> Receiver<SessionKeyFilterStreamResV1> {
        self.filter_update_channel.subscribe()
    }
}

impl Storage {
    pub fn new(
        route_updates: Arc<Sender<RouteStreamResV1>>,
        filter_updates: Arc<Sender<SessionKeyFilterStreamResV1>>,
    ) -> Self {
        let helium_net_id = hex_field::net_id(0xC00053);

        Self {
            orgs: RwLock::new(HashMap::new()),
            routes: RwLock::new(HashMap::new()),
            filters: RwLock::new(HashMap::new()),
            next_oui: Mutex::new(0),
            next_helium_devaddr: Mutex::new(helium_net_id.range_start()),
            route_update_channel: route_updates,
            filter_update_channel: filter_updates,
        }
    }
    fn create_org(&self, org: Org, devaddr_constraints: DevaddrRange) {
        info!(oui = org.oui, "saving org");
        let key = org.oui;

        self.orgs
            .write()
            .expect("org store lock")
            .insert(key, DbOrg::new(org, devaddr_constraints));

        self.routes
            .write()
            .expect("route store lock")
            .insert(key, vec![]);
    }

    fn get_devaddr_constraints(&self, oui: u64) -> Option<DevaddrRange> {
        self.orgs
            .read()
            .expect("org store lock")
            .get(&oui)
            .map(|o| o.devaddr_constraints.to_owned())
    }

    fn ranges_within_org_constraint(&self, oui: u64, ranges: &[DevaddrRange]) -> Result {
        match self.get_devaddr_constraints(oui) {
            Some(constraint) => ranges.iter().all(|range| constraint.contains(range)),
            None => return Err(anyhow!("all orgs should have constraints")),
        };
        Ok(())
    }
}

impl OrgStorage for Storage {
    fn next_oui(&self) -> u64 {
        let mut oui = self.next_oui.lock().expect("could not lock mutex");
        *oui += 1;
        info!(oui = *oui, "next oui");
        *oui
    }

    fn create_helium_org(&self, org: Org, devaddr_constraints: DevaddrRange) {
        let next = devaddr_constraints
            .next_start()
            .expect("next devaddr for net_id");
        let mut helium_devaddr = self.next_helium_devaddr.lock().unwrap();
        *helium_devaddr = next;

        self.create_org(org, devaddr_constraints);
    }

    fn create_roamer_org(&self, org: Org, devaddr_constraints: DevaddrRange) {
        self.create_org(org, devaddr_constraints)
    }

    fn get_orgs(&self) -> Vec<DbOrg> {
        self.orgs
            .read()
            .expect("org store lock")
            .clone()
            .into_values()
            .collect()
    }

    fn get_org(&self, oui: u64) -> Option<DbOrg> {
        self.orgs
            .read()
            .expect("org store lock")
            .clone()
            .get(&oui)
            .map(|i| i.to_owned())
    }
}

impl RouteStorage for Storage {
    fn get_routes(&self, oui: u64) -> Result<Vec<Route>> {
        match self.routes.read().expect("route store lock").get(&oui) {
            Some(routes) => Ok(routes.to_owned()),
            None => Err(anyhow!("org does not exist")),
        }
    }

    fn get_route(&self, route_id: String) -> Option<Route> {
        self.routes
            .read()
            .expect("route store lock")
            .clone()
            .into_values()
            .flatten()
            .find(|route| route_id == route.id)
    }

    fn create_route(&self, oui: u64, route: Route) -> Result<Route> {
        self.ranges_within_org_constraint(oui, &route.devaddr_ranges)?;
        let mut route = route;
        route.id = format!("{}", uuid::Uuid::new_v4());
        let mut store = self.routes.write().expect("route store lock");
        if let Some(routes) = store.get_mut(&oui) {
            let _ = routes.push(route.clone());

            self.route_update_channel.send(RouteStreamResV1 {
                action: ActionV1::Create.into(),
                route: Some(route.clone().into()),
            })?;

            return Ok(route);
        }
        Err(anyhow!("oui does not exist"))
    }

    fn update_route(&self, route: Route) -> Result<Route> {
        self.ranges_within_org_constraint(route.oui, &route.devaddr_ranges)?;
        let mut store = self.routes.write().expect("route store lock");
        if let Some(routes) = store.get_mut(&route.oui) {
            for old_route in routes {
                if old_route.id == route.id {
                    *old_route = route.clone();

                    self.route_update_channel.send(RouteStreamResV1 {
                        action: ActionV1::Update.into(),
                        route: Some(route.clone().into()),
                    })?;

                    return Ok(route);
                }
            }
        }
        Err(anyhow!("could not find route to update"))
    }

    fn delete_route(&self, route_id: String) -> Option<Route> {
        let id_to_remove = route_id;
        // let id_to_remove = String::from_utf8(route_id).expect("valid route id");
        let mut store = self.routes.write().expect("route store lock");
        let removed = store
            .clone()
            .into_values()
            .flatten()
            .find(|old_route| old_route.id == id_to_remove);

        if let Some(inner_route) = &removed {
            if let Some(oui_routes) = store.get_mut(&inner_route.oui) {
                oui_routes.retain(|route| route.id != id_to_remove)
            }
            self.route_update_channel
                .send(RouteStreamResV1 {
                    action: ActionV1::Delete.into(),
                    route: Some(inner_route.clone().into()),
                })
                .expect("sent delete update");
        }

        removed
    }

    fn subscribe_to_routes(&self) -> Receiver<RouteStreamResV1> {
        self.route_update_channel.subscribe()
    }
}

#[derive(Debug, Clone)]
pub struct DbOrg {
    org: Org,
    devaddr_constraints: DevaddrRange,
}

impl DbOrg {
    fn new(org: Org, devaddr_constraints: DevaddrRange) -> Self {
        Self {
            org,
            devaddr_constraints,
        }
    }
}

impl From<DbOrg> for OrgV1 {
    fn from(db: DbOrg) -> Self {
        db.org.into()
    }
}
