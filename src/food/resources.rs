use crate::common::components::Position;

pub struct FoodId {
    pub id: u32
}

// Keep track of positions to eat food that we couldn't delete in the current frame, as it's possible
// the EatFood packet comes before the SpawnFood packet, or within the same frame in which case the
// ECS is not yet updated
pub struct ToEatfood {
    pub positions: Vec<Position>
}