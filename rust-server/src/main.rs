use client_command::client_command::{Room, countDownTime};
use tungstenite::client;
mod client_command;
use std::{
    collections::HashMap,
    env,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex}, thread::current,
};
use tokio::time::{sleep, Duration};

use crate::client_command::client_command::{ClientCommand, Player};
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt, SinkExt};

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::protocol::Message;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", std::process::id());
    let addr = "0.0.0.0:".to_owned()+&env::var("PORT").unwrap_or("8125".to_string());
    
    let state = PeerMap::new(Mutex::new(HashMap::new()));
    let room_map = Arc::new(Mutex::new(HashMap::<String, Room>::new()));
    room_map.lock().unwrap().insert("QuickPlay".to_string(), Room::new("QuickPlay".to_string(), Vec::new()));
    let names = Arc::new(Mutex::new(Vec::<String>::new()));

    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);


    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(process_connection(state.clone(), room_map.clone(), names.clone(), stream, addr));

    }
    
Ok(())
}

#[test]
fn serialize_test() {
    
    let clientCommand = client_command::client_command::ClientCommand::JoinRoom("test".to_string());
    let serialized = serde_json::to_string(&clientCommand).unwrap();
    println!("{}", serialized);
    
    let clientCommand2 = client_command::client_command::ClientCommand::LeaveRoom;
    let serialized2 = serde_json::to_string(&clientCommand2).unwrap();
    println!("{}", serialized2);

    let clientCommand3 = ClientCommand::PostPos(1, 1.0, 1.0);
    let serialized3 = serde_json::to_string(&clientCommand3).unwrap();
    println!("{}", serialized3);

    let clientCommand4 = ClientCommand::QuickPlay;
    let serialized4 = serde_json::to_string(&clientCommand4).unwrap();
    println!("{}", serialized4);

    let room = Room::new("Quick".to_string(), Vec::new());
    let serializedObstacles = serde_json::to_string(&room.obstacles).unwrap();
    println!("{}", serializedObstacles);

}



async fn process_connection(peer_map:PeerMap, RoomMap:Arc<Mutex<HashMap<String, Room>>>,names:Arc<Mutex<Vec<String>>>, raw_stream: TcpStream, addr: SocketAddr){

    let mut current_room = "".to_string();
    let mut socket_index = 0;
    let mut name = "".to_string();
    let mut registered = false;
    let mut curNum = 0;

    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);

    // Insert the write part of this peer to the peer map.
    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(addr, tx);


    let (outgoing, incoming) = ws_stream.split();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        
        println!("Received a message from {}: {}", addr, msg.to_text().unwrap());

        let command = serde_json::from_str::<client_command::client_command::ClientCommand>(msg.to_text().unwrap_or("Error")).unwrap_or(client_command::client_command::ClientCommand::Error);
        //println!("{:?}", command);

        match command{


            ClientCommand::RegPlayer(string) if !registered=> {
                let mut guard = names.lock().unwrap();
                let count = guard.iter().filter(|x| x==&&string).count();
                if count==0 && string.len()>1{
                    guard.push(string.clone());
                    let message = Message::Text("RegPlayer".to_string());
                    peer_map.lock().unwrap().get(&addr).unwrap().unbounded_send(message).unwrap();
                    name = string;
                    registered = true;
                }else{
                    let message = Message::Text("NameTaken".to_string());
                    peer_map.lock().unwrap().get(&addr).unwrap().unbounded_send(message).unwrap();
                }

            },
        ClientCommand::JoinRoom(string) if registered&&current_room==*""=> {
            let mut guard = RoomMap.lock().unwrap();
            let room_map = guard.get_mut(&string);
            let mut ready = false;
            match room_map{
               Some(x) =>{ 
                if x.isStarted{
                    let message = Message::Text("RoomStarted".to_owned());
                    peer_map.lock().unwrap().get(&addr).unwrap().unbounded_send(message).unwrap();    
                }else{

                let pos = x.players.iter().position(|p|p.name.len()<=1);
                if let Some(thing) = pos{
                    if current_room!=string{
                    socket_index= thing;
                    }
                    x.players[socket_index] = client_command::client_command::Player::new(name.clone(), addr);
                }else if current_room!=string{

                socket_index = x.players.len();

                x.players.push(client_command::client_command::Player::new(name.clone(), addr));
               
                }
                current_room = string;

                //TODO: start new thread for room proccessing if it does not exist already
                //ALSO DO QUICKPLAY

                if !x.isReady{
                    //TODO:Fix error
                    x.isReady = true;
                    ready = true;
                    //drop(guard);
                    //drop(room_map);
                    drop(x);
                    tokio::spawn(gameProccessThread(current_room.clone(),peer_map.clone(), RoomMap.clone()));
                }
                let message = Message::Text("RoomJoined".to_owned());
                peer_map.lock().unwrap().get(&addr).unwrap().unbounded_send(message).unwrap();
            }}, 
                _=>{
                    let message = Message::Text("RoomDoesNotExist".to_owned());
                    peer_map.lock().unwrap().get(&addr).unwrap().unbounded_send(message).unwrap();
                },
            }
            
        
        
        },
        ClientCommand::LeaveRoom if registered=> {
            let mut guard = RoomMap.lock().unwrap();
            let room = guard.get_mut(&current_room);
                
            match room{
                Some(x) =>{
                    if socket_index==x.players.len()-1{
                        x.players.remove(socket_index);
                    }else{
                    
                    x.players[socket_index].name="".to_string();
                    }
                },
                _=>{},
            }
        

            current_room = "".to_string();
    
        },
        ClientCommand::QuickPlay if registered&&current_room==*"" => {
            let string = "QuickPlay".to_string();
            let mut guard = RoomMap.lock().unwrap();
            let room_map = guard.get_mut(&string);
            let mut ready = false;
            match room_map{
               Some(x) =>{ 
                if x.isStarted{
                    let message = Message::Text("RoomStarted".to_owned());
                    peer_map.lock().unwrap().get(&addr).unwrap().unbounded_send(message).unwrap();    
                }else{

                let pos = x.players.iter().position(|p|p.name.len()<=1);
                if let Some(thing) = pos {
                    if current_room!=string{
                    socket_index= thing;
                    }
                    x.players[socket_index] = client_command::client_command::Player::new(name.clone(), addr);
                    
                }else if current_room!=string{

                socket_index = x.players.len();

                x.players.push(client_command::client_command::Player::new(name.clone(), addr));
               
                }
                current_room = string;
                println!("Current room {}", &current_room);
                if !x.isReady{
                    //TODO:Fix error
                    x.isReady = true;
                    ready = true;
                    //drop(guard);
                    //drop(room_map);
                    drop(x);
                    tokio::spawn(gameProccessThread(current_room.clone(),peer_map.clone(), RoomMap.clone()));
                }
                let message = Message::Text("RoomJoined".to_owned());
                peer_map.lock().unwrap().get(&addr).unwrap().unbounded_send(message).unwrap();
               }
            }, 
                _=>{
                    //TODO: Make QUICKPLAY if it doesn't exist.
                    let y = Room::new("QuickPlay".to_string(), Vec::new());
                    guard.insert("QuickPlay".to_string(), y);
                    //println!("219 Guard: {:?}", guard);
                    let x = guard.get_mut("QuickPlay").unwrap();    
                    socket_index = x.players.len();
    
                    x.players.push(client_command::client_command::Player::new(name.clone(), addr));
                   
                    current_room = string;
    
                    if !x.isReady{
                        //TODO:Fix error
                        x.isReady = true;
                        ready = true;
                        //drop(guard);
                        //drop(room_map);
                        drop(x);
                        tokio::spawn(gameProccessThread(current_room.clone(),peer_map.clone(), RoomMap.clone()));
                    }
                    let message = Message::Text("RoomJoined".to_owned());
                    peer_map.lock().unwrap().get(&addr).unwrap().unbounded_send(message).unwrap();
                },
            }
            
        
        },
        ClientCommand::GetData if registered&&current_room!="".to_string()=> {
            let mut guard = RoomMap.lock().unwrap();
            let room = guard.get_mut(&current_room);
            match room{
                Some(x) =>{
                    
                    //let player = x.players.get(socket_index).unwrap();
                    let players:Vec<&Player> = x.players.iter().filter(|x| x.name.len()>1).collect();
                    let serde = serde_json::to_string(&players).unwrap();
                    let message = Message::Text("Data!".to_owned()+&serde);
                    //println!("{:?}", message);
                    peer_map.lock().unwrap().get(&addr).unwrap().unbounded_send(message).unwrap();
                    
                },
                _=>{},
            }



        },
        ClientCommand::GetObstacles if registered&&current_room!=*""=>{
            let mut guard = RoomMap.lock().unwrap();
            let room = guard.get_mut(&current_room);
            match room{
                Some(x) =>{
                    
                    //let player = x.players.get(socket_index).unwrap();
                    //let players:Vec<&Player> = x.players.iter().filter(|x| x.name.len()>1).collect();
                    let serde = serde_json::to_string(&x.obstacles).unwrap();
                    let message = Message::Text("Obstacles!".to_owned()+&serde);
                    //println!("{:?}", message);
                    peer_map.lock().unwrap().get(&addr).unwrap().unbounded_send(message).unwrap();
                    
                },
                _=>{},
            }
        },
        ClientCommand::Ready if registered=>{
            let mut guard = RoomMap.lock().unwrap();
            let room = guard.get_mut(&current_room);
            match room{
                Some(x) =>{
                    let player = x.players.get_mut(socket_index);
                    match player{
                        Some(y) => y.isReady = true,
                        _=>{},
                    }
                },
                _=>{},
            }
        },
        ClientCommand::PostPos(num, pos_x,pos_y) if registered =>{
            let mut guard = RoomMap.lock().unwrap();
            let room = guard.get_mut(&current_room);
            match room{
                Some(x) if num > curNum =>{

                    curNum = num;
                    //TODO: anticheat thing?

                    if x.players[socket_index].isReady{
                        x.players[socket_index].x=pos_x;
                        x.players[socket_index].y=pos_y;
                    }
                },
                _=>{
                    //current_room = "".to_string();
                },
            }

        },
        ClientCommand::GetRoomList =>{
            
            let guard = RoomMap.lock().unwrap();
            let serde = serde_json::to_string(&guard.keys().collect::<Vec<&String>>()).unwrap();
            let message = Message::Text("Rooms!".to_owned()+&serde);
            peer_map.lock().unwrap().get(&addr).unwrap().unbounded_send(message).unwrap();

        },
        ClientCommand::CreateRoom(x) if registered&&current_room==*""=>{
            let mut guard = RoomMap.lock().unwrap();
            let room = guard.get_mut(&x);
            match room{
                Some(y) =>{
                    //TODO: Create ROOM
                    let message = Message::Text("RoomAlreadyExists".to_string());
                    peer_map.lock().unwrap().get(&addr).unwrap().unbounded_send(message).unwrap();
                },
                _=>{
                    let y = Room::new(x.clone(), Vec::new());
                    guard.insert(x.clone(), y);
                    let message = Message::Text("RoomCreated".to_string());
                    peer_map.lock().unwrap().get(&addr).unwrap().unbounded_send(message).unwrap();
                },
            }
        
        },
        ClientCommand::Error => println!("Error"),
         _ => println!("Error"),   
        }


        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;



    println!("{} disconnected", &addr);
    peer_map.lock().unwrap().remove(&addr);
    println!("Removed from peerMap");
    println!("Looking for room {}", &current_room);
    if let Some(room) = RoomMap.lock().unwrap().get_mut(&current_room){
        println!("Room found");
        if room.players.len()-1==socket_index{
            room.players.remove(socket_index);
            //TODO: Make culling better somehow
            let mut working = true;
            let mut popdex = 0;
            room.players.iter().rev().for_each(|x| if working && x.name.len()<=1 {popdex+=1;} else {working=false;});
            println!("Pretruncated {:?}", &room.players);
            room.players.truncate(room.players.len()-popdex);
            println!("Truncated {:?}", &room.players);

        }else{
        room.players[socket_index].name="".to_string();
        }
        println!("{}, {}",room.players.len(), peer_map.lock().unwrap().len());
        let count = room.players.iter().position(|x| x.name!="".to_string());
        match count{
            Some(_)=>{},
            None=>room.players.retain(|_|false),
        }

    }
    println!("Names removing");
    names.lock().unwrap().retain(|x|x!=&name);
}

async fn gameProccessThread(cur_room:String,peer_map:PeerMap, RoomMap:Arc<Mutex<HashMap<String, Room>>>){
    println!("Thread Starting");
    let mut totalTime = 20;
    sleep(Duration::from_millis(10000)).await;
    let mut waiting = true;
    let mut addrVec= Vec::new();
//blocks are nice i guess
    
    {
        let rooms = RoomMap.lock();
        if let Ok(x) = rooms{
            if let Some(y) = x.get(&cur_room){
                //let mut count = 0;
                //let mut total = 0;
                y.players.iter().for_each(|x|{addrVec.push(x.addr)});
 
            }
        }

    }


    loop{
        
            if true {
            let rooms = RoomMap.lock();
            if let Ok(x) = rooms{
                if let Some(y) = x.get(&cur_room){
                    let mut count = 0;
                    let mut total = 0;
                    y.players.iter().for_each(|x|{if x.isReady{count+=1} if x.name!=*""{total+=1}});
                    println!("Count: {}, Total: {}, Ratio: {}", count, total, count as f64 / total as f64);
                    if count as f64 / total as f64 > 0.7 {
                        waiting = false;   
                    }
                }
            }
        }
    if waiting{
        println!("WAITING, 349");
        sleep(Duration::from_millis(5000)).await;
    }else{
        //reload room map stuff while here
        {
            let rooms = RoomMap.lock();
            if let Ok(x) = rooms{
                if let Some(y) = x.get(&cur_room){
                    //let mut count = 0;
                    //let mut total = 0;
                    y.players.iter().for_each(|x|{addrVec.push(x.addr)});
     
                }
            }
    
        }
        let serde = serde_json::to_string(&countDownTime::time(totalTime)).unwrap();
        totalTime-=1;
        let message = Message::Text("Countdown!".to_owned()+&serde);
        pres_broadcast(peer_map.clone(), message, &addrVec).await;
        break;
    }


    }
    //better countdown
    let mut playerCount = 1;
    loop{
        {
            let rooms = RoomMap.lock();
            if let Ok(x) = rooms{
                if let Some(y) = x.get(&cur_room){
                    //let mut count = 0;
                    //let mut total = 0;
                    y.players.iter().for_each(|x|{addrVec.push(x.addr)});
     
                }
            }
    
        }

    sleep(Duration::from_millis(1000)).await;
    let serde = serde_json::to_string(&countDownTime::time(totalTime)).unwrap();
    totalTime-=1;
    let message = Message::Text("Countdown!".to_owned()+&serde);
    pres_broadcast(peer_map.clone(), message, &addrVec).await;
    if totalTime<=0{
        break;
    }

//check for new players joining
if true {
    let rooms = RoomMap.lock();
    if let Ok(x) = rooms{
        if let Some(y) = x.get(&cur_room){
            let mut count = 0;
            let mut total = 0;
            y.players.iter().for_each(|x|{if x.isReady{count+=1} if x.name!=*""{total+=1}});
            if total>playerCount{
                totalTime+=5;
                playerCount = total;
            }
        }
    }
}

    }


    let mut truth = false;
    if true{
    let rooms = RoomMap.lock();
    if let Ok(mut x) = rooms{
        if let Some(y) = x.get_mut(&cur_room){
            //TODO: figure out why I put this here
            println!("Room ready: {}",y.isStarted);
            y.isStarted=true;
            truth = true;
        }
    }
    }
    if truth{
        let serde = serde_json::to_string(&countDownTime::start).unwrap();
        let message = Message::Text("Countdown!".to_owned()+&serde);
        pres_broadcast(peer_map.clone(), message,&addrVec).await;
    }

    //DATA SENDING, TODO: REMOVE GET DATA
    loop{
        let mut message = Message::Text("Data! error".to_owned());
        let mut len = -1;
        let mut winner:String = "".to_owned();

        {
        let mut guard = RoomMap.lock().unwrap();
        let room = guard.get_mut(&cur_room);
        
        match room{
            Some(x) =>{
                
                //let player = x.players.get(socket_index).unwrap();
                let players:Vec<&Player> = x.players.iter().filter(|x| x.name.len()>1).collect();
                let serde = serde_json::to_string(&players).unwrap();
                //println!("Players, 427, {:?}", players);
                len = players.len() as i32;
                message = Message::Text("Data!".to_owned()+&serde);
                //println!("{:?}", message);
                let pos = x.players.iter().position(|p| p.x>100000 as f64);
                if let Some(winnerPos) = pos{
                    len = 0;
                    let name = &x.players[winnerPos].name;
                    winner = name.clone();
                }
            },
            _=>{},
        }
        if len==0{
            //TODO: make all other threads return or something
            println!("Deleting room 441");
            guard.remove(&cur_room);
            //break;
        }

        }

        if len==0{
            //TODO: make all other threads return
            println!("Winner: {}", &winner);
            pres_broadcast(peer_map.clone(), Message::Text("GameOver!".to_owned()+&winner),&addrVec).await;
            break;
        }
        pres_broadcast(peer_map.clone(), message, &addrVec).await;  
        sleep(Duration::from_millis(100)).await;
    }

    //TODO: Delete room once game ends/no one is playing.



}

async fn broadcast(peer_map:PeerMap, msg:Message){
        //println!("Broadcasting {}", &msg);

        let peers = peer_map.lock().unwrap();

        let broadcast_recipients =
            peers.iter().map(|(_, ws_sink)| ws_sink);

        for recp in broadcast_recipients {
            recp.unbounded_send(msg.clone()).unwrap();
        }

        

}

async fn pres_broadcast(peer_map:PeerMap, msg:Message, addrVec:&Vec<SocketAddr>){
    //println!("Broadcasting {}", &msg);

    let peers = peer_map.lock().unwrap();

    let broadcast_recipients =
        peers.iter().filter(|x|addrVec.contains(x.0)).map(|(_, ws_sink)| ws_sink);

    for recp in broadcast_recipients {
        recp.unbounded_send(msg.clone()).unwrap();
    }

    

}