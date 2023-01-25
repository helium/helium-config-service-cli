use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, RwLock},
};

use anyhow::anyhow;
use helium_config_service_cli::{
    hex_field::{self, HexDevAddr},
    proto::{DevaddrRangeV1, EuiPairV1, OrgV1},
    route::Route,
    DevaddrConstraint, DevaddrRange, Org, Result, RouteEui,
};
use helium_crypto::PublicKey;
use helium_proto::services::iot_config::{
    route_stream_res_v1, ActionV1, RouteStreamResV1, SessionKeyFilterStreamResV1,
    SessionKeyFilterV1,
};
use tokio::sync::broadcast::{Receiver, Sender};
use tracing::{info, warn};

pub type Oui = u64;
pub type RouteId = String;

pub type OrgMap = RwLock<HashMap<Oui, DbOrg>>;
pub type RouteMap = RwLock<HashMap<Oui, Vec<Route>>>;
pub type Euis = RwLock<HashSet<RouteEui>>;
pub type Devaddrs = RwLock<HashSet<DevaddrRange>>;
pub type Filters = RwLock<HashSet<SessionKeyFilter>>;

#[derive(Debug)]
pub struct Storage {
    orgs: OrgMap,
    routes: RouteMap,
    euis: Euis,
    devaddrs: Devaddrs,
    filters: Filters,
    next_oui: Mutex<u64>,
    next_helium_devaddr: Mutex<HexDevAddr>,
    route_update_channel: Arc<Sender<RouteStreamResV1>>,
    filter_update_channel: Arc<Sender<SessionKeyFilterStreamResV1>>,
}

pub trait OrgStorage {
    fn next_oui(&self) -> u64;
    fn create_helium_org(&self, org: Org, devaddr_constraints: DevaddrConstraint);
    fn create_roamer_org(&self, org: Org, devaddr_constraints: DevaddrConstraint);
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
    // Euis
    fn get_euis_for_route(&self, route_id: &RouteId) -> Vec<EuiPairV1>;
    fn clear_euis_for_route(&self, route_id: &RouteId);
    fn add_eui(&self, eui: EuiPairV1) -> bool;
    fn remove_eui(&self, eui: EuiPairV1) -> bool;
    // Devaddrs
    fn get_devaddrs_for_route(&self, route_id: &RouteId) -> Vec<DevaddrRangeV1>;
    fn clear_devaddrs_for_route(&self, route_id: &RouteId);
    fn add_devaddr(&self, devaddr: DevaddrRangeV1) -> bool;
    fn remove_devaddr(&self, devaddr: DevaddrRangeV1) -> bool;
}

pub trait SkfStorage {
    fn get_filters_for_oui(&self, oui: Oui) -> Result<Vec<SessionKeyFilter>>;
    fn get_filters_for_devaddr(
        &self,
        oui: Oui,
        devaddr: HexDevAddr,
    ) -> Result<Vec<SessionKeyFilter>>;
    fn subscribe_to_filters(&self) -> Receiver<SessionKeyFilterStreamResV1>;
    fn add_filter(&self, filter: SessionKeyFilter) -> bool;
    fn remove_filter(&self, filter: SessionKeyFilter) -> bool;
}

trait RouteUpdate {
    fn notify_add_route(&self, route: Route);
    fn notify_remove_route(&self, route: Route);
    fn notify_add_eui(&self, eui: EuiPairV1);
    fn notify_remove_eui(&self, eui: EuiPairV1);
    fn notify_add_devaddr(&self, devaddr: DevaddrRangeV1);
    fn notify_remove_devaddr(&self, devaddr: DevaddrRangeV1);
    fn notify_add_skf(&self, session_key_filter: SessionKeyFilter);
    fn notify_remove_skf(&self, session_key_filter: SessionKeyFilter);
}

impl RouteUpdate for Storage {
    fn notify_add_route(&self, route: Route) {
        match self.route_update_channel.send(RouteStreamResV1 {
            action: ActionV1::Add.into(),
            data: Some(route_stream_res_v1::Data::Route(route.into())),
        }) {
            Ok(count) => info!("route add sent to {count} receivers"),
            Err(_err) => info!("no one is listening"),
        };
    }

    fn notify_remove_route(&self, route: Route) {
        match self.route_update_channel.send(RouteStreamResV1 {
            action: ActionV1::Remove.into(),
            data: Some(route_stream_res_v1::Data::Route(route.into())),
        }) {
            Ok(count) => info!("route remove sent to {count} receivers"),
            Err(_err) => info!("no one is listening"),
        };
    }

    fn notify_add_eui(&self, eui: EuiPairV1) {
        match self.route_update_channel.send(RouteStreamResV1 {
            action: ActionV1::Add.into(),
            data: Some(route_stream_res_v1::Data::EuiPair(eui)),
        }) {
            Ok(count) => info!("eui add sent to {count} receivers"),
            Err(_err) => info!("no one is listening"),
        };
    }

    fn notify_remove_eui(&self, eui: EuiPairV1) {
        match self.route_update_channel.send(RouteStreamResV1 {
            action: ActionV1::Remove.into(),
            data: Some(route_stream_res_v1::Data::EuiPair(eui)),
        }) {
            Ok(count) => info!("eui remove sent to {count} receivers"),
            Err(_err) => info!("no one is listening"),
        };
    }

    fn notify_add_devaddr(&self, devaddr: DevaddrRangeV1) {
        match self.route_update_channel.send(RouteStreamResV1 {
            action: ActionV1::Add.into(),
            data: Some(route_stream_res_v1::Data::DevaddrRange(devaddr)),
        }) {
            Ok(count) => info!("devaddr add sent to {count} receivers"),
            Err(_err) => info!("no one is listening"),
        };
    }

    fn notify_remove_devaddr(&self, devaddr: DevaddrRangeV1) {
        match self.route_update_channel.send(RouteStreamResV1 {
            action: ActionV1::Remove.into(),
            data: Some(route_stream_res_v1::Data::DevaddrRange(devaddr)),
        }) {
            Ok(count) => info!("devaddr remove sent to {count} receivers"),
            Err(_err) => info!("no one is listening"),
        };
    }

    fn notify_add_skf(&self, session_key_filter: SessionKeyFilter) {
        match self
            .filter_update_channel
            .send(SessionKeyFilterStreamResV1 {
                action: ActionV1::Add.into(),
                filter: Some(session_key_filter.into()),
            }) {
            Ok(count) => info!("skf add sent to {count} receivers"),
            Err(_err) => todo!("no one is listening"),
        }
    }

    fn notify_remove_skf(&self, session_key_filter: SessionKeyFilter) {
        match self
            .filter_update_channel
            .send(SessionKeyFilterStreamResV1 {
                action: ActionV1::Remove.into(),
                filter: Some(session_key_filter.into()),
            }) {
            Ok(count) => info!("skf remove sent to {count} receivers"),
            Err(_err) => info!("no one is listening"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionKeyFilter {
    oui: u64,
    devaddr: HexDevAddr,
    session_key: PublicKey,
}

impl From<SessionKeyFilterV1> for SessionKeyFilter {
    fn from(filter: SessionKeyFilterV1) -> Self {
        Self {
            oui: filter.oui,
            devaddr: (filter.devaddr as u64).into(),
            session_key: PublicKey::try_from(filter.session_key)
                .expect("valid public key for session key filter"),
        }
    }
}

impl From<SessionKeyFilter> for SessionKeyFilterV1 {
    fn from(filter: SessionKeyFilter) -> Self {
        Self {
            oui: filter.oui,
            devaddr: filter.devaddr.0 as u32,
            session_key: filter.session_key.into(),
        }
    }
}

impl SkfStorage for Storage {
    fn get_filters_for_oui(&self, oui: Oui) -> Result<Vec<SessionKeyFilter>> {
        Ok(self
            .filters
            .read()
            .expect("euis store lock")
            .clone()
            .into_iter()
            .filter_map(|filter| {
                if filter.oui == oui {
                    Some(filter)
                } else {
                    None
                }
            })
            .collect())
    }

    fn get_filters_for_devaddr(
        &self,
        oui: Oui,
        devaddr: HexDevAddr,
    ) -> Result<Vec<SessionKeyFilter>> {
        Ok(self
            .filters
            .read()
            .expect("filter store lock")
            .clone()
            .into_iter()
            .filter_map(|filter| {
                if filter.oui == oui && filter.devaddr == devaddr {
                    Some(filter)
                } else {
                    None
                }
            })
            .collect())
    }

    fn add_filter(&self, filter: SessionKeyFilter) -> bool {
        let added = self
            .filters
            .write()
            .expect("filter write lock")
            .insert(filter.clone().into());

        if added {
            self.notify_add_skf(filter);
        }

        added
    }

    fn remove_filter(&self, filter: SessionKeyFilter) -> bool {
        let removed = self
            .filters
            .write()
            .expect("filter write lock")
            .remove(&filter.clone().into());

        if removed {
            self.notify_remove_skf(filter);
        }

        removed
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
            euis: RwLock::new(HashSet::new()),
            devaddrs: RwLock::new(HashSet::new()),
            filters: RwLock::new(HashSet::new()),
            next_oui: Mutex::new(0),
            next_helium_devaddr: Mutex::new(helium_net_id.range_start()),
            route_update_channel: route_updates,
            filter_update_channel: filter_updates,
        }
    }
    fn create_org(&self, org: Org, devaddr_constraints: DevaddrConstraint) {
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

    fn get_devaddr_constraints(&self, oui: u64) -> Option<DevaddrConstraint> {
        self.orgs
            .read()
            .expect("org store lock")
            .get(&oui)
            .map(|o| o.devaddr_constraints.to_owned())
    }

    fn ranges_within_org_constraint(&self, oui: Oui, ranges: &[DevaddrRange]) -> Result<bool> {
        match self.get_devaddr_constraints(oui) {
            Some(constraint) => Ok(ranges.iter().all(|range| constraint.contains(range))),
            None => return Err(anyhow!("all orgs should have constraints")),
        }
    }

    fn get_org_for_route_id(&self, route_id: RouteId) -> Oui {
        let route = self.get_route(route_id).expect("route exists");
        route.oui
    }
}

impl OrgStorage for Storage {
    fn next_oui(&self) -> u64 {
        let mut oui = self.next_oui.lock().expect("could not lock mutex");
        *oui += 1;
        info!(oui = *oui, "next oui");
        *oui
    }

    fn create_helium_org(&self, org: Org, devaddr_constraints: DevaddrConstraint) {
        let next = devaddr_constraints
            .next_start()
            .expect("next devaddr for net_id");
        let mut helium_devaddr = self.next_helium_devaddr.lock().unwrap();
        *helium_devaddr = next;

        self.create_org(org, devaddr_constraints);
    }

    fn create_roamer_org(&self, org: Org, devaddr_constraints: DevaddrConstraint) {
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
        let mut route = route;
        route.id = format!("{}", uuid::Uuid::new_v4());
        let mut store = self.routes.write().expect("route store lock");
        if let Some(routes) = store.get_mut(&oui) {
            routes.push(route.clone());

            self.notify_add_route(route.clone());
            return Ok(route);
        }
        Err(anyhow!("oui does not exist"))
    }

    fn update_route(&self, route: Route) -> Result<Route> {
        let mut store = self.routes.write().expect("route store lock");
        if let Some(routes) = store.get_mut(&route.oui) {
            for old_route in routes {
                if old_route.id == route.id {
                    *old_route = route.clone();

                    self.notify_add_route(route.clone());
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
            self.notify_remove_route(inner_route.clone());
        }

        removed
    }

    fn subscribe_to_routes(&self) -> Receiver<RouteStreamResV1> {
        self.route_update_channel.subscribe()
    }

    fn get_euis_for_route(&self, route_id: &RouteId) -> Vec<EuiPairV1> {
        self.euis
            .read()
            .expect("euis store lock")
            .clone()
            .into_iter()
            .filter_map(|eui_pair| {
                if &eui_pair.route_id == route_id {
                    Some(eui_pair.into())
                } else {
                    None
                }
            })
            .collect()
    }

    fn clear_euis_for_route(&self, route_id: &RouteId) {
        let to_remove = self
            .euis
            .read()
            .expect("euis store lock")
            .clone()
            .into_iter()
            .filter(|eui_pair| &eui_pair.route_id == route_id);

        for eui in to_remove {
            self.remove_eui(eui.into());
        }
    }

    fn add_eui(&self, eui: EuiPairV1) -> bool {
        let added = self
            .euis
            .write()
            .expect("euis store lock")
            .insert(eui.clone().into());

        if added {
            self.notify_add_eui(eui);
        }

        added
    }

    fn remove_eui(&self, eui: EuiPairV1) -> bool {
        let removed = self
            .euis
            .write()
            .expect("euis store lock")
            .remove(&eui.clone().into());

        if removed {
            self.notify_remove_eui(eui);
        }

        removed
    }

    fn get_devaddrs_for_route(&self, route_id: &RouteId) -> Vec<DevaddrRangeV1> {
        self.devaddrs
            .read()
            .expect("devaddrs store lock")
            .clone()
            .into_iter()
            .filter_map(|devaddr| {
                if &devaddr.route_id == route_id {
                    Some(devaddr.into())
                } else {
                    None
                }
            })
            .collect()
    }

    fn clear_devaddrs_for_route(&self, route_id: &RouteId) {
        let to_remove = self
            .devaddrs
            .read()
            .expect("devaddrs store lock")
            .clone()
            .into_iter()
            .filter(|devaddr| &devaddr.route_id == route_id);

        for devaddr in to_remove {
            self.remove_devaddr(devaddr.into());
        }
    }

    fn add_devaddr(&self, devaddr: DevaddrRangeV1) -> bool {
        let oui = self.get_org_for_route_id(devaddr.route_id.clone());
        let range: DevaddrRange = devaddr.clone().into();
        match self.ranges_within_org_constraint(oui, &vec![range]) {
            Ok(true) => {
                let added = self
                    .devaddrs
                    .write()
                    .expect("devaddrs store lock")
                    .insert(devaddr.clone().into());
                if added {
                    self.notify_add_devaddr(devaddr);
                }
                added
            }
            Ok(false) => {
                warn!("devaddr outside org constraint");
                false
            }
            Err(e) => {
                warn!("cannot add devaddr: {e:?}");
                false
            }
        }
    }

    fn remove_devaddr(&self, devaddr: DevaddrRangeV1) -> bool {
        let removed = self
            .devaddrs
            .write()
            .expect("devaddrs store lock")
            .remove(&devaddr.clone().into());

        if removed {
            self.notify_remove_devaddr(devaddr);
        }

        removed
    }
}

#[derive(Debug, Clone)]
pub struct DbOrg {
    org: Org,
    pub devaddr_constraints: DevaddrConstraint,
}

impl DbOrg {
    fn new(org: Org, devaddr_constraints: DevaddrConstraint) -> Self {
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
