pub struct Season {
    pub id: SeasonID,
    pub name: String,
    pub episodes: Vec<Episode>,
}

pub struct Episode {
    pub number: u32,
    pub description: String,
}

pub struct SeasonID {
    pub ani_list: Option<u32>,
}
