pub mod client_command{


use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::{TryStreamExt, SplitSink}, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::WebSocketStream;
use tungstenite::protocol::Message;

use rand::Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct ClientCommand{
pub name: String,
pub args: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ObstacleList{

pub obstacles: Vec<Obstacle>,
pub game_id: u128,

}

impl ObstacleList{

fn new(game_id: u128, len: u64, start: u64) -> ObstacleList{
let mut obstacles: Vec<Obstacle> = Vec::new();


let mut done = false;
let mut x = start;
let mut rng = rand::thread_rng();

while !done {

obstacles.push(Obstacle{x: x, y: 0, obstacle_type: rng.gen_range(0..26)});
x+=rng.gen_range(300..1300);
if x > len {
done = true;
}
}

return ObstacleList{obstacles: obstacles, game_id};

}

}

#[derive(Serialize, Deserialize)]
pub struct Obstacle{

x: u64, y: u64, obstacle_type: u64,

}

pub struct ServerData{

usernames: Vec<String>,
game_ids: Vec<u128>,
games: Vec<Game>,
ingame: Vec<String>,


}
impl ServerData{

pub fn new() -> ServerData{
ServerData{usernames: Vec::new(), game_ids: Vec::new(), games: Vec::new(), ingame: Vec::new()}

}
pub fn addPlayer(&mut self, username: String) -> Result<(), JoinError>{

//check if username is in players
if !self.usernames.contains(&username){
self.usernames.push(username);

return Ok(());
}else{
    return Err(JoinError::UsernameTaken);
}


}

pub fn joinGame(&mut self, username: String, gameID: u128) -> Result<(), JoinError>{

    //check if player is in a game
    if self.ingame.contains(&username){
        return Err(JoinError::InGame);
    }
    let game = self.getGame(gameID)?;
        
    //check if game is full
    if game.is_full(){
        return Err(JoinError::GameFull);
    }
    //check if game is started
    if game.status==GameStatus::Playing{
        return Err(JoinError::GameStarted);
    }


    //add player to game
    game.add_player(&username);
    self.ingame.push(username.clone());
    

    Err(JoinError::Unknown)
}

pub fn getGame(&mut self, gameID: u128) -> Result<&mut Game, JoinError>{

    for game in &mut self.games{
        if game.game_id == gameID{
            return Ok(game);
        }
    }

    Err(JoinError::GameDoesNotExist)

}
pub fn leaveGame(&mut self, username: String, gameID: u128) -> Result<(), JoinError>{

let _ = &mut self.ingame.retain(|x|x!=&username);
let _ = &self.getGame(gameID)?.usernames.retain(|x|x!=&username);
Ok(())
}

pub fn newGame(&mut self){
    let mut rng = rand::thread_rng();
    loop{
    let gameId = self.game_ids.iter().max().unwrap()+rng.gen_range(500..10000);
    if !self.game_ids.contains(&gameId){
        self.game_ids.push(gameId);
        self.games.push(Game{usernames: Vec::new(), game_id: gameId, obstacles: ObstacleList::new(gameId, 100000, 1048), size: 400, status: GameStatus::Waiting});
        break;
        }
    }
}
}



#[derive(Error, Debug)]
pub enum JoinError{
    #[error("Username is already taken")]
    UsernameTaken,
    #[error("Game is full")]
    GameFull,
    #[error("Player is already in a game")]
    InGame,
    #[error("Game has already started")]
    GameStarted,
    #[error("Game does not exist")]
    GameDoesNotExist,
    #[error("Username is not allowed")]
    UsernameNotAllowed,
    #[error("Unknown error")]
    Unknown,
}


pub struct Game{

usernames: Vec<String>,
game_id: u128,
pub obstacles: ObstacleList,
size: i32,
status: GameStatus,

}

impl Game{
    fn is_full(&self) -> bool{
        if self.usernames.len() as i32 >= self.size{
            return true;
        }
        false
    }

    fn add_player(&mut self, username: &String){
        
        self.usernames.push(username.to_string());
    }
    pub fn start(&mut self){
        self.status = GameStatus::Playing;
    }
    pub fn end(&mut self){
        self.status = GameStatus::Finished;
    }
}

#[derive(Debug, PartialEq)]
pub enum GameStatus{ 
        Waiting,
        Playing,
        Finished,
}

pub struct SocketData{

pub address: String,
pub username: String,
pub socket: SplitSink<WebSocketStream<TcpStream>, Message>,
pub current_game: u128,

}







}

