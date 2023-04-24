pub mod client_command {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum ClientCommand {
        JoinRoom(String),
        LeaveRoom,
        QuickPlay,
        GetData,
        GetRoomList,
    }

    pub struct Room {
        pub name: String,
        pub players: Vec<Player>,
        pub maxPlayers: u8,
        pub isPrivate: bool,
        pub password: String,
        pub isPlaying: bool,
        pub isFull: bool,
        pub isStarted: bool,
        pub isFinished: bool,
        pub isWaiting: bool,
        pub isReady: bool,
        pub isPaused: bool,
        pub isEnded: bool,
        pub isCancelled: bool,
        pub isAbandoned: bool,
        pub isSpectating: bool,
        pub isSpectatingAllowed: bool,
        pub isSpectatingFull: bool,
        pub playerAdjust: Vec<Vec<Player>>,
    }

    impl Room{
        fn new(name:String, players: Vec<Player>) -> Room{
            Room{
                name,
                players,
                maxPlayers: 0,
                isPrivate: false,
                password: String::new(),
                isPlaying: false,
                isFull: false,
                isStarted: false,
                isFinished: false,
                isWaiting: false,
                isReady: false,
                isPaused: false,
                isEnded: false,
                isCancelled: false,
                isAbandoned: false,
                isSpectating: false,
                isSpectatingAllowed: false,
                isSpectatingFull: false,
                playerAdjust: Vec::new(),

            }
        }


    }


    pub struct Player {
        pub name: String,
        pub isReady: bool,
        pub isPlaying: bool,
        pub isFinished: bool,
        pub x: f64,
        pub y: f64,
        pub speed: f64,   
        
    }

    impl Player{
        fn new(name:String) -> Player{
            Player{
                name,
                isReady: false,
                isPlaying: false,
                isFinished: false,
                x: 0.0,
                y: 0.0,
                speed: 3.0,
            }
        }
    }


}
