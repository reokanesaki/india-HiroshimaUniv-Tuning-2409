use super::dto::tow_truck::TowTruckDto;
use super::map_service::MapRepository;
use super::order_service::OrderRepository;
use crate::errors::AppError;
use crate::models::graph::Graph;
use crate::models::tow_truck::TowTruck;

//ここから追加
/*
use std::sync::Arc;
use tokio::sync::Mutex;
*/
//ここまで追加

pub trait TowTruckRepository {
    async fn get_paginated_tow_trucks(
        &self,
        page: i32,
        page_size: i32,
        status: Option<String>,
        area_id: Option<i32>,
    ) -> Result<Vec<TowTruck>, AppError>;
    async fn update_location(&self, truck_id: i32, node_id: i32) -> Result<(), AppError>;
    async fn update_status(&self, truck_id: i32, status: &str) -> Result<(), AppError>;
    async fn find_tow_truck_by_id(&self, id: i32) -> Result<Option<TowTruck>, AppError>;
}

#[derive(Debug)]
pub struct TowTruckService<
    T: TowTruckRepository + std::fmt::Debug,
    U: OrderRepository + std::fmt::Debug,
    V: MapRepository + std::fmt::Debug,
> {
    tow_truck_repository: T,
    order_repository: U,
    map_repository: V,
}

impl<
        T: TowTruckRepository + std::fmt::Debug,
        U: OrderRepository + std::fmt::Debug,
        V: MapRepository + std::fmt::Debug,
    > TowTruckService<T, U, V>
{   
    //追加
    //#[inline(always)]
    pub fn new(tow_truck_repository: T, order_repository: U, map_repository: V) -> Self {
        TowTruckService {
            tow_truck_repository,
            order_repository,
            map_repository,
        }
    }

    //追加
    //#[inline(always)]
    pub async fn get_tow_truck_by_id(&self, id: i32) -> Result<Option<TowTruckDto>, AppError> {
        let tow_truck = self.tow_truck_repository.find_tow_truck_by_id(id).await?;
        Ok(tow_truck.map(TowTruckDto::from_entity))
    }

    pub async fn get_all_tow_trucks(
        &self,
        page: i32,
        page_size: i32,
        status: Option<String>,
        area: Option<i32>,
    ) -> Result<Vec<TowTruckDto>, AppError> {
        let tow_trucks = self
            .tow_truck_repository
            .get_paginated_tow_trucks(page, page_size, status, area)
            .await?;
        let tow_truck_dtos = tow_trucks
            .into_iter()
            .map(TowTruckDto::from_entity)
            .collect();

        Ok(tow_truck_dtos)
    }

    //追加
    //#[inline(always)]
    pub async fn update_location(&self, truck_id: i32, node_id: i32) -> Result<(), AppError> {
        self.tow_truck_repository
            .update_location(truck_id, node_id)
            .await?;

        Ok(())
    }

    // 元の関数
    pub async fn get_nearest_available_tow_trucks(
        &self,
        order_id: i32,
    ) -> Result<Option<TowTruckDto>, AppError> {
        let order = self.order_repository.find_order_by_id(order_id).await?;
        let area_id = self
            .map_repository
            .get_area_id_by_node_id(order.node_id)
            .await?;
        let tow_trucks = self
            .tow_truck_repository
            .get_paginated_tow_trucks(0, -1, Some("available".to_string()), Some(area_id))
            .await?;

        let nodes = self.map_repository.get_all_nodes(Some(area_id)).await?;
        let edges = self.map_repository.get_all_edges(Some(area_id)).await?;

        let mut graph = Graph::new();
        for node in nodes {
            graph.add_node(node);
        }
        for edge in edges {
            graph.add_edge(edge);
        }

        let sorted_tow_trucks_by_distance = {
            let mut tow_trucks_with_distance: Vec<_> = tow_trucks
                .into_iter()
                .map(|truck| {
                    let distance = calculate_distance(&graph, truck.node_id, order.node_id);
                    (distance, truck)
                })
                .collect();

            tow_trucks_with_distance.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            tow_trucks_with_distance
        };

        if sorted_tow_trucks_by_distance.is_empty() || sorted_tow_trucks_by_distance[0].0 > 10000000
        {
            return Ok(None);
        }

        let sorted_tow_truck_dtos: Vec<TowTruckDto> = sorted_tow_trucks_by_distance
            .into_iter()
            .map(|(_, truck)| TowTruckDto::from_entity(truck))
            .collect();

        Ok(sorted_tow_truck_dtos.first().cloned())
    }

    /* 
    //ここから追加した部分    
    pub async fn get_nearest_available_tow_trucks(
        &self,
        order_id: i32,
    ) -> Result<Option<TowTruckDto>, AppError> {
        // 注文情報の取得
        let order = self.order_repository.find_order_by_id(order_id).await?;
        
        // エリアIDの取得
        let area_id = self
            .map_repository
            .get_area_id_by_node_id(order.node_id)
            .await?;
        
        // 利用可能なレッカー車の取得
        let tow_trucks = self
            .tow_truck_repository
            .get_paginated_tow_trucks(0, usize::MAX, Some("available".to_string()), Some(area_id))
            .await?;
        
        // エリア内のノードとエッジの取得
        let nodes = self.map_repository.get_all_nodes(Some(area_id)).await?;
        let edges = self.map_repository.get_all_edges(Some(area_id)).await?;
        
        // グラフの初期化とラップ
        let graph = Arc::new(Mutex::new(Graph::new()));
        
        // グラフへのノードとエッジの追加を並列で実行
        let graph_clone1 = Arc::clone(&graph);
        let graph_clone2 = Arc::clone(&graph);
        
        tokio::join!(
            async {
                let mut graph = graph_clone1.lock().await;
                for node in &nodes {
                    graph.add_node(node.clone());
                }
            },
            async {
                let mut graph = graph_clone2.lock().await;
                for edge in &edges {
                    graph.add_edge(edge.clone());
                }
            }
        );
        
        // ArcとMutexを解放してGraphを取得
        let graph = Arc::try_unwrap(graph)
            .map_err(|_| AppError::GraphError("Graph is still referenced".into()))?
            .into_inner();
        
        // レッカー車の距離計算とソート
        let mut tow_trucks_with_distance: Vec<_> = tow_trucks
            .into_iter()
            .map(|truck| {
                let distance = calculate_distance(&graph, truck.node_id, order.node_id);
                (distance, truck)
            })
            .collect();
    
        tow_trucks_with_distance.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        
        // 距離の閾値チェック
        const MAX_DISTANCE: f64 = 10_000_000.0;
        if tow_trucks_with_distance.is_empty() || tow_trucks_with_distance[0].0 > MAX_DISTANCE {
            log::warn!("No available tow trucks found within distance {}", MAX_DISTANCE);
            return Ok(None);
        }
        
        // レッカー車DTOへの変換
        let sorted_tow_truck_dtos: Vec<TowTruckDto> = tow_trucks_with_distance
            .into_iter()
            .map(|(_, truck)| TowTruckDto::from_entity(truck))
            .collect();
        
        // 最も近いレッカー車の返却
        Ok(sorted_tow_truck_dtos.first().cloned())
    }
    */
    //ここまで追加した部分 
    
}

fn calculate_distance(graph: &Graph, node_id_1: i32, node_id_2: i32) -> i32 {
    graph.shortest_path(node_id_1, node_id_2)
}
