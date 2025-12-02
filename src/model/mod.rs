use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct Artist {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct Album {
    pub id: i64,
    pub title: String,
    pub artist_id: Option<i64>,
    pub year: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Track {
    pub id: i64,
    pub title: String,
    pub artist_id: Option<i64>,
    pub album_id: Option<i64>,
    pub path: String,
    pub duration: Option<i64>,
    pub track_number: Option<i64>,
}
