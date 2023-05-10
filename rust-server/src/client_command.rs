pub mod client_command {


    use serde::{Deserialize, Serialize};
    use rand::prelude::*;
    const obstacleTypes:[obstacle_type; 13] = [obstacle_type::Cactus, obstacle_type::Cactus1,obstacle_type::Cactus2,obstacle_type::Cactus3,obstacle_type::Cactus4,obstacle_type::Cactus5,obstacle_type::Cactus6,obstacle_type::Cactus7,obstacle_type::Cactus8,obstacle_type::Cactus9,obstacle_type::Cactus10,obstacle_type::Cactus11,obstacle_type::Cactus12];

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
        GetObstacles,
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
        pub obstacles: Vec<obstacle>,
        

    }

    impl Room{
        pub fn new(name:String, players: Vec<Player>) -> Room{
            let mut obstacleList = Vec::new();
                    //gameSeed = generateTerrain(100000);
            let length = 100000;
            let mut obstacle_distance = 1000;
            let mut i = 1106;
            let mut rng = rand::thread_rng();
            loop{
                obstacle_distance = rng.gen_range(0..1000)+400;
                
                let obstacleVariation = rng.gen_range(0..obstacleTypes.len());


                obstacleList.push(obstacle::new(obstacleTypes[obstacleVariation].clone(), i));

                //console.log("added new obstacle at "+i+","+0);
                i+=obstacle_distance;
                if i>=length{
                    break;
                }
            }      

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
                length: length,
                obstacles: obstacleList,
            }
        }


    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum obstacle_type{
        Cactus,
        Cactus1,
        Cactus2,
        Cactus3,
        Cactus4,
        Cactus5,
        Cactus6,
        Cactus7,
        Cactus8,
        Cactus9,
        Cactus10,
        Cactus11,
        Cactus12,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct obstacle{
        obstacleType:obstacle_type,
        xPos:u64,
    }

    impl obstacle{
        fn new(obstacleType:obstacle_type, xPos:u64)->obstacle{
            return obstacle{obstacleType, xPos};
        }

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
