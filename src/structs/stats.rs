use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]

pub struct Stats {
    pub resrouces: Option<StatsResources>,
    pub performance: Option<StatPerformance>,
}
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct StatPerformance {
    pub bucket: Option<i32>,
    pub usage: Option<f64>,
    pub limit: Option<u32>,
    pub max: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct StatsResources {
    pub pixel: Option<u128>,
    pub cpu: Option<u128>,
    pub credits: Option<u128>,
}
