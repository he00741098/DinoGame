pub mod client_command {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum ClientCommand {
        RegPlayer(String),
        JoinRoom(String),
        //posnum, x, y
        PostPos(i32, f64, f64),
        LeaveRoom,
        QuickPlay,
        GetData,
        GetRoomList,
        Ready,
        Error,
    }

    pub struct Room {
        pub name: String,
        pub players: Vec<Player>,
        pub maxPlayers: u8,
        pub isPrivate: bool,
        pub password: String,
        pub isFull: bool,
        pub isStarted: bool,
        pub isFinished: bool,
        pub isReady: bool,
        pub isEnded: bool,
        pub isSpectatingAllowed: bool,
        pub playerAdjust: Vec<Vec<Player>>,
        pub length:u64,

    }

    impl Room{
        pub fn new(name:String, players: Vec<Player>) -> Room{
            Room{
                name,
                players,
                maxPlayers: 0,
                isPrivate: false,
                password: String::new(),
                isFull: false,
                isStarted: false,
                isFinished: false,
                isReady: false,
                isEnded: false,
                isSpectatingAllowed: false,
                playerAdjust: Vec::new(),
                length: 100000,
            }
        }


    }

    pub enum obstacle{
        Cactus1(u64),
        Cactus2(u64),
        Cactus3(u64),
        Cactus4(u64),
        Cactus5(u64),
        Bird1(u64),
        Bird2(u64),
    }


#[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Player {
        pub name: String,
        pub isReady: bool,
        pub isFinished: bool,
        pub x: f64,
        pub y: f64,
        pub speed: f64,   
        
    }

    impl Player{
        pub fn new(name:String) -> Player{
            Player{
                name,
                isReady: false,
                isFinished: false,
                x: 0.0,
                y: 0.0,
                speed: 3.0,
            }
        }
    }


}
