use client_command::client_command::Room;
use tungstenite::client;
mod client_command;
use std::{
    collections::HashMap,
    env,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

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
    let addr = "127.0.0.1:".to_owned()+&env::var("PORT").unwrap_or("8125".to_string());
    
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

}



async fn process_connection(peer_map:PeerMap, RoomMap:Arc<Mutex<HashMap<String, Room>>>,names:Arc<Mutex<Vec<String>>>, raw_stream: TcpStream, addr: SocketAddr){

    let mut current_room = "".to_string();
    let mut socket_index = 0;
    let mut name = "".to_string();
    let mut registered = false;

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
        ClientCommand::JoinRoom(string) if registered=> {
            let mut guard = RoomMap.lock().unwrap();
            let room_map = guard.get_mut(&string);
            match room_map{
               Some(x) =>{ 
                socket_index = x.players.len();
                x.players.push(client_command::client_command::Player::new(name.clone()));
               current_room = string;
               
            }, 
                _=>{},
            }
            
        
        
        },
        ClientCommand::LeaveRoom if registered=> {
            let mut guard = RoomMap.lock().unwrap();
            let room = guard.get_mut(&current_room);
            match room{
                Some(x) =>{
                    x.players.remove(socket_index);
                },
                _=>{},
            }
            current_room = "".to_string();
    
        },
        ClientCommand::QuickPlay if registered=> {
            let string = "QuickPlay".to_string();
            let mut guard = RoomMap.lock().unwrap();
            let room_map = guard.get_mut(&string);
            match room_map{
               Some(x) =>{ 
                socket_index = x.players.len();
                x.players.push(client_command::client_command::Player::new(name.clone()));
               current_room = string;
               
            }, 
                _=>{},
            }
            
        
        },
        ClientCommand::GetData if registered=> {
            let mut guard = RoomMap.lock().unwrap();
            let room = guard.get_mut(&current_room);
            match room{
                Some(x) =>{
                    
                    //let player = x.players.get(socket_index).unwrap();
                    let players:Vec<&Player> = x.players.iter().filter(|x| x.name.len()>1).collect();
                    let serde = serde_json::to_string(&players).unwrap();
                    let message = Message::Text(serde);
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
        ClientCommand::Error => println!("Error"),
         _ => println!("Error"),   
        }

        //let peers = peer_map.lock().unwrap();

        // We want to broadcast the message to everyone except ourselves.
        // let broadcast_recipients =
        //     peers.iter().filter(|(peer_addr, _)| peer_addr != &&addr).map(|(_, ws_sink)| ws_sink);

        // for recp in broadcast_recipients {
        //     recp.unbounded_send(msg.clone()).unwrap();
        // }

        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;



    println!("{} disconnected", &addr);
    peer_map.lock().unwrap().remove(&addr);
    //RoomMap.lock().unwrap().get_mut(&current_room).unwrap().players.remove(socket_index);
    if let Some(room) = RoomMap.lock().unwrap().get_mut(&current_room){
        room.players[socket_index].name="".to_string();
        println!("{}",room.players.len());
    }
    names.lock().unwrap().retain(|x|x!=&name);
}