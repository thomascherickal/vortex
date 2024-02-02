use std::collections::HashMap;

use crate::array::stats::{Stat, StatsCompute, StatsSet};
use crate::array::zigzag::ZigZagArray;

impl StatsCompute for ZigZagArray {
    fn compute(&self, _stat: &Stat) -> StatsSet {
        // TODO(ngates): implement based on the encoded array
        StatsSet::from(HashMap::new())
    }
}
