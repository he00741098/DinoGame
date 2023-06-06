use client_command::client_command::{Room, countDownTime};
use tungstenite::client;
mod client_command;
//mod shuttle_service;
use std::{
    collections::HashMap,
    env,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex}, thread::current, f32::consts::E, fs::read,
};
use tokio::time::{sleep, Duration};

use crate::client_command::client_command::{ClientCommand, Player};
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt, SinkExt};

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::protocol::Message;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;


//#[tokio::main]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
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

async fn new_run(addr: std::net::SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", std::process::id());
    //let addr = "0.0.0.0:".to_owned()+&env::var("PORT").unwrap_or("8125".to_string());
    
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
    peer_map.lock().expect("Peer lock error 91").insert(addr, tx);


    let (outgoing, incoming) = ws_stream.split();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        
        //println!("Received a message from {}: {}", addr, msg.to_text().expect("TO Text error 98"));

        let command = serde_json::from_str::<client_command::client_command::ClientCommand>(msg.to_text().unwrap_or("Error")).unwrap_or(client_command::client_command::ClientCommand::Error);
        //println!("{:?}", command);

        match command{


            ClientCommand::RegPlayer(string) if !registered=> {
                let mut guard = names.lock().expect("Name lock error 107");
                let count = guard.iter().filter(|x| x==&&string).count();
                if count==0 && string.len()>1{
                    guard.push(string.clone());
                    let message = Message::Text("RegPlayer".to_string());
                    peer_map.lock().expect("Peer map lock error 112").get(&addr).expect("Get error 112").unbounded_send(message).expect("Send error 112");
                    name = string;
                    registered = true;
                }else{
                    let message = Message::Text("NameTaken".to_string());
                    peer_map.lock().expect("Peer map lock error 117").get(&addr).expect("Get error 117").unbounded_send(message).expect("Send error 117");
                }

            },
        ClientCommand::JoinRoom(string) if registered&&current_room==*""=> {
            let mut guard = RoomMap.lock().expect("Room lock error 122");
            let room_map = guard.get_mut(&string);
            let mut ready = false;
            match room_map{
               Some(x) =>{ 
                if x.isStarted{
                    let message = Message::Text("RoomStarted".to_owned());
                    peer_map.lock().expect("Peer lock error 129").get(&addr).expect("Peer lock error 129").unbounded_send(message).expect("Send error 129");    
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
                    println!("Dropping x");
                    drop(x);
                    tokio::spawn(gameProccessThread(current_room.clone(),peer_map.clone(), RoomMap.clone()));
                }
                let message = Message::Text("RoomJoined".to_owned());
                peer_map.lock().expect("Peer lock error 160").get(&addr).expect("Get error 160").unbounded_send(message).expect("Send error 160");
            }}, 
                _=>{
                    let message = Message::Text("RoomDoesNotExist".to_owned());
                    peer_map.lock().expect("Peer lock error 164").get(&addr).expect("Get error 164").unbounded_send(message).expect("Get error 164");
                },
            }
            
        
        
        },
        ClientCommand::LeaveRoom if registered=> {
            let mut guard = RoomMap.lock().expect("Room map lock error 172");
            let room = guard.get_mut(&current_room);
            //ready = false;
            match room{
                Some(x) =>{
                    if socket_index==x.players.len()-1{
                        x.players.remove(socket_index);
                    }else{
                    x.players[socket_index].name="".to_string();
                    }
                },
                _=>{
                    println!("Room not found 185");
                },
            }
        

            current_room = "".to_string();
    
        },
        ClientCommand::QuickPlay if registered&&current_room==*"" => {
            let string = "QuickPlay".to_string();
            let mut guard = RoomMap.lock().expect("Lock error 193");
            let room_map = guard.get_mut(&string);
            let mut ready = false;
            match room_map{
               Some(x) =>{ 
                if x.isStarted{
                    let message = Message::Text("RoomStarted".to_owned());
                    peer_map.lock().expect("Lock error 200").get(&addr).expect("Get error 200").unbounded_send(message).expect("Send error 200");    
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
                    println!("Dropping x 225");
                    drop(x);
                    tokio::spawn(gameProccessThread(current_room.clone(),peer_map.clone(), RoomMap.clone()));
                }
                let message = Message::Text("RoomJoined".to_owned());
                peer_map.lock().expect("Peer Lock error 229").get(&addr).expect("Get error 229").unbounded_send(message).expect("Send error 229");
               }
            }, 
                _=>{
                    //TODO: Make QUICKPLAY if it doesn't exist.
                    println!("QuickPlay Does Not exist, creating QuickPlay");
                    let y = Room::new("QuickPlay".to_string(), Vec::new());
                    guard.insert("QuickPlay".to_string(), y);
                    //println!("219 Guard: {:?}", guard);
                    if let Some(x) = guard.get_mut("QuickPlay"){   
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
                    peer_map.lock().expect("Peer map lock 254").get(&addr).expect("Get error 254").unbounded_send(message).expect("Send Error 254");
                }else{println!("QuickPlay not found 256");}
                },
            }
            
        
        },
        ClientCommand::GetData if registered&&current_room!="".to_string()=> {
            let mut guard = RoomMap.lock().expect("Lock error 262");
            let room = guard.get_mut(&current_room);
            match room{
                Some(x) =>{
                    
                    //let player = x.players.get(socket_index).unwrap();
                    let players:Vec<&Player> = x.players.iter().filter(|x| x.name.len()>1).collect();
                    let serde = serde_json::to_string(&players).expect("Parse error 269");
                    let message = Message::Text("Data!".to_owned()+&serde);
                    //println!("{:?}", message);
                    peer_map.lock().expect("Peer lock 272").get(&addr).expect("Get error 272").unbounded_send(message).expect("Send error 272");
                    
                },
                _=>{println!("Room not found 275");},
            }



        },
        ClientCommand::GetObstacles if registered&&current_room!=*""=>{
            let mut guard = RoomMap.lock().expect("Room lock error 282");
            let room = guard.get_mut(&current_room);
            match room{
                Some(x) =>{
                    
                    //let player = x.players.get(socket_index).unwrap();
                    //let players:Vec<&Player> = x.players.iter().filter(|x| x.name.len()>1).collect();
                    let serde = serde_json::to_string(&x.obstacles).expect("Parse error 289");
                    let message = Message::Text("Obstacles!".to_owned()+&serde);
                    //println!("{:?}", message);
                    peer_map.lock().expect("Peer lock error 292").get(&addr).expect("Get error 292").unbounded_send(message).expect("Send error 292");
                    
                },
                _=>{println!("Room not found 295");},
            }
        },
        ClientCommand::Ready if registered=>{
            let mut guard = RoomMap.lock().expect("Lock error 299");
            let room = guard.get_mut(&current_room);
            match room{
                Some(x) =>{
                    let player = x.players.get_mut(socket_index);
                    match player{
                        Some(y) => y.isReady = true,
                        _=>{println!("Player not found 306");},
                    }
                },
                _=>{println!("Room not found 309");
                },
            }
        },
        ClientCommand::PostPos(num, pos_x,pos_y) if registered =>{
            let mut guard = RoomMap.lock().expect("Room map lock 313");
            let room = guard.get_mut(&current_room);
            match room{
                Some(x) if num > curNum =>{

                    curNum = num;
                    //TODO: anticheat thing?

                    if socket_index<x.players.len() && x.players[socket_index].isReady{
                        x.players[socket_index].x=pos_x;
                        x.players[socket_index].y=pos_y;
                    }
                },
                _=>{
                    println!("Soft Error: Room not found 328");
                },
            }

        },
        ClientCommand::GetRoomList =>{
            
            let guard = RoomMap.lock().expect("Lock error 334");
            let serde = serde_json::to_string(&guard.keys().collect::<Vec<&String>>()).expect("Parse Error 335");
            let message = Message::Text("Rooms!".to_owned()+&serde);
            peer_map.lock().expect("Peer lock error 337").get(&addr).expect("Get error 337").unbounded_send(message).expect("Send error 337");

        },
        ClientCommand::CreateRoom(x) if registered&&current_room==*""=>{
            let mut guard = RoomMap.lock().expect("Room map Lock error 341");
            let room = guard.get_mut(&x);
            match room{
                Some(y) =>{
                    //TODO: Create ROOM
                    let message = Message::Text("RoomAlreadyExists".to_string());
                    peer_map.lock().expect("Peer Map lock error 347").get(&addr).expect("Can't find addr 347").unbounded_send(message).expect("Send error 347");
                },
                _=>{
                    let y = Room::new(x.clone(), Vec::new());
                    guard.insert(x.clone(), y);
                    let message = Message::Text("RoomCreated".to_string());
                    peer_map.lock().expect("Peer Map error 353").get(&addr).expect("Can't find addr 353").unbounded_send(message).expect("Send error 353");
                },
            }
        
        },
        ClientCommand::Error => println!("Error Command"),
         _ => println!("Error, Default"),   
        }


        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;



    println!("{} disconnected", &addr);
    peer_map.lock().expect("Peermap lock error").remove(&addr);
    println!("Removed from peerMap");
    println!("Looking for room {}", &current_room);
    if let Some(room) = RoomMap.lock().expect("Room Lock error").get_mut(&current_room){
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
        println!("{}, {}",room.players.len(), peer_map.lock().expect("Peer map error 392").len());
        let count = room.players.iter().position(|x| x.name!="".to_string());
        match count{
            Some(_)=>{println!("Room empty 395");},
            None=>room.players.retain(|_|false),
        }

    }else{println!("Room not found 399");}
    println!("Names removing 400");
    names.lock().expect("Can't lock names 401").retain(|x|x!=&name);
}

async fn gameProccessThread(cur_room:String,peer_map:PeerMap, RoomMap:Arc<Mutex<HashMap<String, Room>>>){
    println!("Thread Starting");
    let mut totalTime = 20;
    sleep(Duration::from_millis(10000)).await;
    let mut waiting = true;
    let mut addrVec= Vec::new();
    let mut playerCount = 0;
//blocks are nice i guess
    
    {
        let rooms = RoomMap.lock();
        if let Ok(x) = rooms{
            if let Some(y) = x.get(&cur_room){
                //let mut count = 0;
                //let mut total = 0;
                y.players.iter().for_each(|x|{addrVec.push(x.addr)});
 
            }else{println!("Room not found 421");}
        }else{println!("No Map 422");}

    }

    let mut waitRounds = 0;
    loop{
        
            if true {
            let rooms = RoomMap.lock();
            if let Ok(mut x) = rooms{
                if let Some(y) = x.get(&cur_room){
                    let mut count = 0;
                    let mut total = 0;
                    y.players.iter().for_each(|x|{if x.isReady{count+=1} if x.name!=*""{total+=1}});
                    println!("Count: {}, Total: {}, Ratio: {}", count, total, count as f64 / total as f64);
                    if count as f64 / total as f64 > 0.7 {
                        waiting = false;   
                        playerCount = total;
                    }else{
                        if total ==0{
                            waiting = false;
                            playerCount=0;
                            //println!("Deleting room 441");
                            x.remove(&cur_room);
                            return;
                        }
                    }
                    
                }else{println!("Room not found 442");}
            }else{println!("Map not found 443");}
        }
    if waiting{
        println!("WAITING, 349");
        sleep(Duration::from_millis(5000)).await;
        waitRounds+=1;
        if waitRounds>10{
            println!("starting game");
            waiting = false;
        }
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
            }else{
                println!("No Room 460");
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
    
    loop{
        {
            let rooms = RoomMap.lock();
            if let Ok(x) = rooms{
                if let Some(y) = x.get(&cur_room){
                    //let mut count = 0;
                    //let mut total = 0;
                    y.players.iter().for_each(|x|{addrVec.push(x.addr)});
     
                }else{println!("No room 495");}
            }else{println!("Error rooms 495");}
    
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
        let mut guard = RoomMap.lock().expect("Hash Map Lock Erro 540");
        let room = guard.get_mut(&cur_room);
        
        match room{
            Some(x) =>{
                
                //let player = x.players.get(socket_index).unwrap();
                let players:Vec<&Player> = x.players.iter().filter(|x| x.name.len()>1).collect();
                let serde = serde_json::to_string(&players).expect("Error 548");
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
            _=>{
                println!("Room Not Found 561");
            },
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

        let peers = peer_map.lock().expect("PeerMap Error 590");

        let broadcast_recipients =
            peers.iter().map(|(_, ws_sink)| ws_sink);

        for recp in broadcast_recipients {
            recp.unbounded_send(msg.clone()).expect("Recp Broadcast Error 596");
        }

        

}

async fn pres_broadcast(peer_map:PeerMap, msg:Message, addrVec:&Vec<SocketAddr>){
    //println!("Broadcasting {}", &msg);

    let peers = peer_map.lock().expect("PeerMap Error 606");

    let broadcast_recipients =
        peers.iter().filter(|x|addrVec.contains(x.0)).map(|(_, ws_sink)| ws_sink);

    for recp in broadcast_recipients {
        recp.unbounded_send(msg.clone()).expect("Recp Broadcast Error 612");
    }

    

}


pub struct Custom_Service{
    
    
}

#[shuttle_runtime::async_trait]
impl shuttle_runtime::Service for Custom_Service {
    async fn bind(
        mut self,
        addr: std::net::SocketAddr,
    ) -> Result<(), shuttle_runtime::Error> {
        
        //let router = self.router.into_inner();

        //let serve_router = axum::Server::bind(&addr).serve(router.into_make_service());

        //tokio::spawn(async move {
            //serve_router.await.expect("Server Error 624");
        new_run(addr).await.expect("Server Error 624");
        

        Ok(())
    }
}

#[shuttle_runtime::main]
async fn init() -> Result<Custom_Service, shuttle_runtime::Error> {

Ok(Custom_Service{
    
})

}