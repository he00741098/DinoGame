document.getElementById("inputer").style.display = "block";
document.getElementById("GamePage").style.display = "none";

//wss://rustdinogame.herokuapp.com/
const host = "wss://rustdinogame.herokuapp.com/";
const container = document.getElementById("container");
const socket = new WebSocket(host);
const deathSound = new Audio("./sounds/hit.mp3");
const jumpSound = new Audio("./sounds/press.mp3");
const scoreSound = new Audio("./sounds/reached.mp3");
let username = "";
let box = document.getElementById("usernamebox");
var devMode = false;
var mapSprites = [];
var dead = false;
let deathText;
let gameLoop_interval;
let cloudLoop_interval;
let scoreLoop_interval;
let socketRenew_interval;
//let countdown_interval;
let countdown_Time;
//let table_interval;
let countingDown = false;
let w = 512, h=512;
var ratio;
var table = new Tabulator("#score-table", {
    height:"300px",
    layout:"fitDataTable",
    initialSort:[{column:"score", dir:"desc"}],
    columns:[{title:"Username", field:"username"},{title:"Score", field:"score", sorter: "number"}]
});

table.on("tableBuilt", () => {
    table.setPage(2);
});

const image = new Image();
image.src = '/images/DinoSprites.png';
const floorImage = new Image();
floorImage.src = '/images/floor.png';
const atlasData = {
	frames: {
		Dino1: {
			frame: { x: 0, y:0, w:60, h:60 },
			sourceSize: { w: 32, h: 32 },
			spriteSourceSize: { x: 0, y: 0, w: 32, h: 32 }
		},
		Dino2: {
			frame: { x: 60, y:0, w:60, h:60 },
			sourceSize: { w: 32, h: 32 },
			spriteSourceSize: { x: 0, y: 0, w: 32, h: 32 }
		},
        DuckDino1: {
            frame: { x: 240, y:0, w: 60, h:60 },
            sourceSize: {w: 32, h:32},
            spriteSourceSize: {x: 0, y: 0, w: 32, h: 32}
        },
        DuckDino2: {
            frame: {x: 0, y:60, w:60, h:60},
            sourceSize: {w: 32, h:32},
            spriteSourceSize: {x: 0, y: 0, w: 32, h:32}

        },
        JumpDino1:{
            frame: {x:120, y:0, w:60, h:60},
            sourceSize: {w: 32, h:32},
            spriteSourceSize: {x: 0, y: 0, w: 32, h:32}
        },
        JumpDino2:{
            frame: {x:180, y:0, w:60, h:60},
            sourceSize: {w: 32, h:32},
            spriteSourceSize: {x: 0, y: 0, w: 32, h:32}
        },
	    floor: {
		    frame: {x:180, y: 240, w:60, h:60},
		    sourceSize: {w: 32, h:32},
		    spriteSourceSize: {x: 0, y:0, w:32, h:32}
	    },		
        cloud: {
            frame: {x: 120, y: 230, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        cactus: {
            frame: {x: 0, y:180, w:60, h:60},
            sourceSize: {w: 32, h:32},
            spriteSourceSize: {x: 0, y: 0, w: 32, h:32}

        },
        cactus2: {
            frame: {x: 60, y:180, w:60, h:60},
            sourceSize: {w: 32, h:32},
            spriteSourceSize: {x: 0, y: 0, w: 32, h:32}

        },
        cactus3: {
            frame: {x: 120, y:180, w:60, h:60},
            sourceSize: {w: 32, h:32},
            spriteSourceSize: {x: 0, y: 0, w: 32, h:32}

        },
        cactus4: {
            frame: {x: 180, y:180, w:60, h:60},
            sourceSize: {w: 32, h:32},
            spriteSourceSize: {x: 0, y: 0, w: 32, h:32}

        },
        cactus5: {
            frame: {x: 0, y:120, w:60, h:60},
            sourceSize: {w: 32, h:32},
            spriteSourceSize: {x: 0, y: 0, w: 32, h:32}

        },
        cactus6: {
            frame: {x: 60, y:120, w:60, h:60},
            sourceSize: {w: 32, h:32},
            spriteSourceSize: {x: 0, y: 0, w: 32, h:32}

        },
        cactus7: {
            frame: {x: 120, y:120, w:60, h:60},
            sourceSize: {w: 32, h:32},
            spriteSourceSize: {x: 0, y: 0, w: 32, h:32}

        },
        cactus8: {
            frame: {x: 180, y:120, w:60, h:60},
            sourceSize: {w: 32, h:32},
            spriteSourceSize: {x: 0, y: 0, w: 32, h:32}

        },
        cactus9: {
            frame: {x: 240, y:120, w:60, h:60},
            sourceSize: {w: 32, h:32},
            spriteSourceSize: {x: 0, y: 0, w: 32, h:32}

        },
        cactus10: {
            frame: {x: 120, y:60, w:60, h:60},
            sourceSize: {w: 32, h:32},
            spriteSourceSize: {x: 0, y: 0, w: 32, h:32}

        },
        cactus11: {
            frame: {x: 180, y:60, w:60, h:60},
            sourceSize: {w: 32, h:32},
            spriteSourceSize: {x: 0, y: 0, w: 32, h:32}

        },
        cactus12: {
            frame: {x: 240, y:60, w:60, h:60},
            sourceSize: {w: 32, h:32},
            spriteSourceSize: {x: 0, y: 0, w: 32, h:32}

        }

	},
	meta: {
		image: image,
		format: 'RGBA8888',
		size: { w: 128, h: 32 },
		scale: 1
	},
	animations: {
		dino: ['Dino1','Dino2'], //array of frames by name
        duckDino: ['DuckDino1', 'DuckDino2'],
        jumpDino: ['JumpDino1', 'JumpDino2'],
        cactus: ['cactus'],
        cactus2: ['cactus2'],
        cactus3: ['cactus3'],
        cactus4: ['cactus4'],
        cactus5: ['cactus5'],
        cactus6: ['cactus6'],
        cactus7: ['cactus7'],
        cactus8: ['cactus8'],
        cactus9: ['cactus9'],
        cactus10: ['cactus10'],
        cactus11: ['cactus11'],
        cactus12: ['cactus12'],
        // oh wait theres only 12  >:[
        // cactus13: ['cactus13'],
        // cactus14: ['cactus14'],
        // cactus15: ['cactus15'],
        // cactus16: ['cactus16'],
        // cactus17: ['cactus17'],
        // cactus18: ['cactus18'],
        // cactus19: ['cactus19'],
        // cactus20: ['cactus20'],
        cloud: ['cloud'],
	    floor: ['floor']
	}
}
const floorData ={
    frames: {
        floor1: {
            frame: {x: 0, y: 0, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor2: {
            frame: {x: 60, y: 0, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor3: {
            frame: {x: 120, y: 0, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor4: {
            frame: {x: 180, y: 0, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor5: {
            frame: {x: 240, y: 0, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor6: {
            frame: {x: 0, y: 60, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor7: {
            frame: {x: 60, y: 60, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor8: {
            frame: {x: 120, y: 60, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor9: {
            frame: {x: 180, y: 60, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor10: {
            frame: {x: 240, y: 60, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor11: {
            frame: {x: 0, y: 120, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor12: {
            frame: {x: 60, y: 120, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor13: {
            frame: {x: 120, y: 120, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor14: {
            frame: {x: 180, y: 120, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor15: {
            frame: {x: 240, y: 120, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor16: {
            frame: {x: 0, y: 180, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor17: {
            frame: {x: 60, y: 180, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor18: {
            frame: {x: 120, y: 180, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor19: {
            frame: {x: 180, y: 180, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor20: {
            frame: {x: 240, y: 180, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor21: {
            frame: {x: 0, y: 240, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor22: {
            frame: {x: 60, y: 240, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor23: {
            frame: {x: 120, y: 240, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor24: {
            frame: {x: 180, y: 240, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        },
        floor25: {
            frame: {x: 240, y: 240, w:60, h:60},
            sourceSize: {w:32, h:32},
            spriteSourceSize: {x:0,y:0,w:32,h:32}
        }
	},
	meta: {
		image: floorImage,
		format: 'RGBA8888',
		size: { w: 128, h: 32 },
		scale: 1
	},
	animations: {
        floor1:['floor1'],
        floor2:['floor2'],
        floor3:['floor3'],
        floor4:['floor4'],
        floor5:['floor5'],
        floor6:['floor6'],
        floor7:['floor7'],
        floor8:['floor8'],
        floor9:['floor9'],
        floor10:['floor10'],
        floor11:['floor11'],
        floor12:['floor12'],
        floor13:['floor13'],
        floor14:['floor14'],
        floor15:['floor15'],
        floor16:['floor16'],
        floor17:['floor17'],
        floor18:['floor18'],
        floor19:['floor19'],
        floor20:['floor20'],
        floor21:['floor21'],
        floor22:['floor22'],
        floor23:['floor23'],
        floor24:['floor24'],
        floor25:['floor25']
	}
}
const spritesheet = new PIXI.Spritesheet(PIXI.BaseTexture.from(atlasData.meta.image),atlasData);
const floorsheet = new PIXI.Spritesheet(PIXI.BaseTexture.from(floorData.meta.image),floorData);
spritesheet.parse();
floorsheet.parse();
let speedup = 1.5;
let pos = 0;
let clouds = [];
let anim = new PIXI.AnimatedSprite(spritesheet.animations.dino);
anim.animationSpeed = 0.1666;
anim.play();
let app = new PIXI.Application({width: 1106, height: 310});
app.renderer.backgroundColor = 0xffffff;
app.stage.sortableChildren = true;
let scoreText = new PIXI.Text("0", {fontFamily: 'Arial', fontSize: 24, fill: "black", align: 'right'});
const obstacles = [];
var thing;
let floor = [];
let index = 0;
let using = [];
let started = false;
let floorDex = 0;
let pressed = {};
pressed['holding']=false;
pressed['holdingDown'] = false;
let postNum = 0;
let spriteY = 285;



//register player
function reg(){
    
    if(box.value == "") {
        alert("please input a username");
        return;
    }
    username = box.value;
    socket.send(JSON.stringify({"RegPlayer":box.value}));

}



let tableData = [];
socket.onmessage = function (Event){

onMessager(Event);

}

function onMessager(Event){
    switch(Event.data){
        case "RegPlayer":
            console.log("Registered");

            socket.send(JSON.stringify("QuickPlay"));
            socket.send(JSON.stringify("Ready"));
            socket.send(JSON.stringify("GetObstacles"));
            document.getElementById("inputer").style.display = "none";
            document.getElementById("GamePage").style.display = "block";
            deathText = new PIXI.Text("Waiting for game to start...", {fontFamily: 'Arial', fontSize: 24, fill: "black", align: 'right'});
            deathText.anchor.x = -1;
            deathText.anchor.y = -5;
            app.stage.addChild(deathText);


            break;
        case "NameTaken":
            //box.value = "";
            console.log("NameTaken");
            alert("Name taken");
            break;
        default:
            let type = Event.data.slice(0, Event.data.indexOf("!"));
            let data = "";
            try{
            data = JSON.parse(Event.data.slice(Event.data.indexOf("!")+1));
            }catch{
                console.log("O_O");
                data="";
            }
            //console.log(type +", "+data);
            switch(type){
                case "Data":
                //console.log(data);
                tableData = [];
                for(let res of data){
                    if(res['name']==username) {
                        //TODO: Maybe do dynamic thingies
                        player.score = Math.floor(res['x']/70);
                        tableData.push({username : res['name'], score: player.score});
                        continue;
                    }
                    if(mapSprites[res['name']]!=null) {
                        mapSprites[res['name']].x= res['x']*ratio;
                    }else{
                        mapSprites[res['name']]= new PIXI.AnimatedSprite(spritesheet.animations.dino);
                        mapSprites[res['name']].x= res['x']*ratio;
                        mapSprites[res['name']].y = spriteY;
                        mapSprites[res['name']].width = 16;
                        mapSprites[res['name']].height = 16;
                        mapSprites[res['name']].tint = "#d9ad4e";
                        app.stage.addChild(mapSprites[res['name']]);
                    }
                    tableData.push({username : res['name'], score: Math.floor(res['x']/70)});
                }
                updateTable();

                break;
                case "Obstacles":
                    thing = data;
                    for(let i of thing){
                    //TODO: add different types of obstacles and also object pool
                    let obstacle_type = i["obstacleType"];
                    //= new PIXI.AnimatedSprite(spritesheet.animations.cactus)
                    let sprite;
                    switch(obstacle_type){
                        case "Cactus":
                        sprite = new PIXI.AnimatedSprite(spritesheet.animations.cactus);
                        break;
                        case "Cactus2":
                        sprite = new PIXI.AnimatedSprite(spritesheet.animations.cactus2);
                        break;
                        case "Cactus3":
                            sprite = new PIXI.AnimatedSprite(spritesheet.animations.cactus3);
                            break;
                        case "Cactus4":
                            sprite = new PIXI.AnimatedSprite(spritesheet.animations.cactus4);
                            break;
                        case "Cactus5":
                            sprite = new PIXI.AnimatedSprite(spritesheet.animations.cactus5);
                            break;
                        case "Cactus6":
                            sprite = new PIXI.AnimatedSprite(spritesheet.animations.cactus6);
                            break;
                        case "Cactus7":
                            sprite = new PIXI.AnimatedSprite(spritesheet.animations.cactus7);
                            break;
                        case "Cactus8":
                            sprite = new PIXI.AnimatedSprite(spritesheet.animations.cactus8);
                            break;
                        case "Cactus9":
                            sprite = new PIXI.AnimatedSprite(spritesheet.animations.cactus9);
                            break;
                        case "Cactus10":
                            sprite = new PIXI.AnimatedSprite(spritesheet.animations.cactus10);
                            break;
                        case "Cactus11":
                            sprite = new PIXI.AnimatedSprite(spritesheet.animations.cactus11);
                            break;
                        case "Cactus12":
                            sprite = new PIXI.AnimatedSprite(spritesheet.animations.cactus12);
                            break;
                        default:
                            sprite =new PIXI.AnimatedSprite(spritesheet.animations.cactus);
                    }



                    let defY = (h/4);
                    sprite.x = app.view.width;
                    sprite.y = defY-sprite.height;


                    obstacles.push(new obstacle(i['xPos'], 0, 2, 5, sprite));
                }
                //    startGame();
                break;
                case "Countdown":
                    switch(data){
                        case "start":
                            app.stage.removeChild(deathText);
                            startGame();
                        break;
                        case "stopped":
                            //TODO: implement stop, gameover thing
                        break;
                        default:
                        console.log(data);
                        //let obj = JSON.parse(data);
                        if (data["time"] !=null){
                            countdown_Time = data["time"];
                            //countdown_interval = setInterval(countdown, 1000);
                            countdown();
                        }
                        break;    

                    }
                break;
                case "GameOver":
                    //gameLoop_interval = setInterval(gameLoop, 1000/60);
                    //cloudLoop_interval = setInterval(cloudLoop, 1000/30);
                    //scoreLoop_interval = setInterval(scoreLoop, 100);
                    clearInterval(gameLoop_interval);
                    clearInterval(scoreLoop_interval);
                    
                    deathText.text = Event.data.slice(Event.data.indexOf("!")+1)+" Wins!";
                    deathText.anchor.x=-3;
                    deathText.anchor.y=-2.9;
                    deathText.width*=1.5;
                    deathText.height*=1.5;
                    app.stage.addChild(deathText);
                    break;
            }
            break;
    }
}

//counts down with countdown_Time variable

function countdown(){
    console.log("Time Left: " + countdown_Time);
    socket.send(JSON.stringify("GetData"));
    if(countdown_Time==0){
        deathText.text = "Starting...";
        //clearInterval(countdown_interval);
    }
    else if (!countingDown){
        countingDown = true;
        deathText.text = deathText.text + countdown_Time;
    console.log("Countdown Started");
    }
    else{
        //console.log("Counting Down: "+(deathText.text.length-((countingDown+1)+"").length));
        //;-; - console.log("Debugging: "+(""+(countingDown+1)).length+" "+(""+(countingDown+1)) +"\n"+"Total len: "+deathText.text.length);
        deathText.text = deathText.text.slice(0, deathText.text.length-(""+(countdown_Time+1)).length) + countdown_Time;
    }
    //countdown_Time--;

}

window.onload = function (){
    container.appendChild(app.view);
    anim.anchor.set(0.5);
    anim.x = app.view.width / 6;
    anim.y = (app.view.height / 2)+10;
    app.stage.addChild(anim);
    app.stage.addChild(scoreText);
    anim.zIndex = 5;
    //app.stage.addChild(cloud);
    //put dino on "map"
    //TODO: look at app ticker


}

function getRandomInt(min, max) {
    min = Math.ceil(min);
    max = Math.floor(max);
    return Math.floor(Math.random() * (max - min + 1)) + min;
}

//more variables for game


function cloudLoop() {
    for(let c of clouds) {
        c.x-=(4*speedup);
        if(c.x<-100) {
            c.x=1106;
        }
    }
}

function gameLoop() {
    //TODO: better speedup;
    speedup+=0.001;
    if(clouds.length<getRandomInt(3,5)) {
        for(let i = 0; i<getRandomInt(3,5); i++) {
            let cloud = new PIXI.AnimatedSprite(spritesheet.animations.cloud);
            clouds.push(cloud);
        }

        let x = 0;
        for(let c of clouds) {
            app.stage.addChild(c);
            c.x=x;
            let randomsize=getRandomInt(60,70)
            c.width=randomsize;
            c.height=randomsize;
            x+=getRandomInt(100,500);
            c.y=getRandomInt(-50,30);
        }
    }
    
        //TODO: FLOOOOOOOOOOOR!!!!!!!!!!!!!!!
    if(floor.length<5){
            for(let g = 0; g<25; g++){
                let floorSprite;// = new PIXI.AnimatedSprite(spritesheet.animations.floor);
                let randomFloor = Math.floor(Math.random()*26);
                if(randomFloor == 0){
                    floorSprite = new PIXI.AnimatedSprite(spritesheet.animations.floor);
                }else if(randomFloor ==1){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor1);
                }else if(randomFloor ==2){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor2);
                }else if(randomFloor ==3){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor3);
                }else if(randomFloor ==4){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor4);
                }else if(randomFloor ==5){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor5);
                }else if(randomFloor ==6){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor6);
                }else if(randomFloor ==7){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor7);
                }else if(randomFloor ==8){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor8);
                }else if(randomFloor ==9){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor9);
                }else if(randomFloor ==10){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor10);
                }else if(randomFloor ==11){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor11);
                }else if(randomFloor ==12){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor12);
                }else if(randomFloor ==13){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor13);
                }else if(randomFloor ==14){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor14);
                }else if(randomFloor ==15){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor15);
                }else if(randomFloor ==16){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor16);
                }else if(randomFloor ==17){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor17);
                }else if(randomFloor ==18){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor18);
                }else if(randomFloor ==19){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor19);
                }else if(randomFloor ==20){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor20);
                }else if(randomFloor ==21){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor21);
                }else if(randomFloor ==22){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor22);
                }else if(randomFloor ==23){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor23);
                }else if(randomFloor ==24){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor24);
                }else if(randomFloor ==25){
                    floorSprite = new PIXI.AnimatedSprite(floorsheet.animations.floor25);
                }else{
                    floorSprite = new PIXI.AnimatedSprite(spritesheet.animations.floor);
                }
                floor.push(floorSprite);
        
            }
             let x = 0;
            for(let p of floor){
                //console.log("Adding");
                app.stage.addChild(p);
                p.x=x;
                x+=50;
                p.y = (app.view.height / 2)+5;
            }
        }


	
    player.update();
    //pos++;
    //checks if obstacles should be moved I think
    if(obstacles.length>0&&(!started||obstacles[index].x<pos+1106)){
        if(!started){
            //TODO: show a map of all the player positions, also make a width variable
            ratio = 1106/obstacles[obstacles.length-1]["x"];
            //console.log("the ration:" + ratio);
            let mapSprite = new PIXI.AnimatedSprite(spritesheet.animations.dino);
            //mapSprite.height= 60;
            //mapSprite.width= 30;
            mapSprite.x = pos*ratio;
            //console.log("mapSpriteX:"+mapSprite.x);
            mapSprite.y = spriteY;
            mapSprite.zIndex = 4;
            mapSprite.width = 16;
            mapSprite.height = 16;
            app.stage.addChild(mapSprite);
            mapSprites[username] = mapSprite;

            //console.log("added map sprite");

        }


        started = true;
        obstacles[index].sprite.width = 60;
        obstacles[index].sprite.height = 75;
        obstacles[index].x = app.view.width;
        obstacles[index].sprite.x = app.view.width;
        app.stage.addChild(obstacles[index].sprite);
        using.push(obstacles[index]);
        if(index<obstacles.length-1){
        index++;
    }
    }

    move(using);
}
//move function - to be used by game loop to move obstacles

//let usingFloor = [];

function move(obstacle_list){
    for(i = obstacle_list.length-1; i>=0; i--){
        if(obstacle_list[i].sprite.x<-220){
            floorDex+=1;
            if(floorDex>=floor.length){
                floorDex = 0;
            }

            using = using.splice(0, i).concat(using.splice(i+1));
            obstacle_list = using;
            
        } else {
            //TODO: Slow down player instead of death in multiplayer game
            //TODO: Fix death when player reaches end of map. Instead show "you win" or something
            //console.log(obstacle_list[i].sprite);
            if(boxesIntersect(obstacle_list[i].sprite, anim)){
                // dead = true;
                // clearInterval(gameLoop_interval);
                // clearInterval(scoreLoop_interval);
                // clearInterval(cloudLoop_interval);
                // window.removeEventListener("keydown", onkeydown);
                // window.removeEventListener("keyup", onkeyup);
                // anim.stop();
                
                //socket.close();
                obstacle_list[i].sprite.x=-200;
                speedup*=0.8;
                
                //app.stage.removeChild(obstacle_list[i.sprite]);
                // deathText = new PIXI.Text("You died! -- Press ENTER to retry", {fontFamily: 'Arial', fontSize: 24, fill: "black", align: 'right'});
                // deathText.anchor.x = -1;
                // deathText.anchor.y = -5;
                // app.stage.addChild(deathText);
                // window.addEventListener("keydown", deathKey);
            
            }

            //obstacles being moved across the screen
            obstacle_list[i].x-=(4*speedup);
            obstacle_list[i].sprite.x-=(4*speedup);
 
        }
    }
    for(let o of floor){
        o.x-=(4*speedup);
        if(o.x<-100){
            o.x=1106;
        }
    }

}

//array for movement handling

//player class
class Player{
    constructor(color, radius, v) {
        this.radius = radius;
        this.v = v;
        this.circle = anim;
        this.defX = app.view.width / 6;
        this.defY = anim.y = (app.view.height / 2)+10;
        this.maxHeight = 25;
        this.score = 0;
        this.reset();
    }

    reset() {
        this.circle.x = w/2;
        this.circle.y = h/2;
        this.speed = 4*speedup;
    }

    update() {
        this.speed = 4*speedup;
        //TODO: send pos to server
        pos+=this.speed;
        //console.log(pos);

        if(this.circle.y<this.maxHeight){
            //pressed['up'] = false;
            this.v.y=this.speed;
            //console.log("too high");
            anim.stop();
            anim.textures = spritesheet.animations.dino;
            anim.play();
            pressed['holding'] = true;
        }
        if(this.circle.y>this.defY){
            //pressed['down'] = false;
            this.v.y=0;
            this.circle.y = this.defY;
            pressed['holding'] = false;
            //console.log("too low");
        }
        //console.log(this.circle.y);
        let x = this.circle.x + this.v.x;
        let y = this.circle.y + this.v.y;

        this.circle.x = Math.min(Math.max(x, this.radius), w-this.radius);
        this.circle.y = Math.min(Math.max(y, this.radius), w-this.radius);
        if((this.score % 100) == 0) {
            scoreSound.play();
        }
    }
}
// moves on keydown
function onkeydown(ev) {
    switch (ev.key) {
        case "ArrowUp":
        case "Spacebar":
        //spacebar is " " apparently
        case " ":
        case "w":
            jumpSound.play();
            if(player.circle.y==player.defY) {
                player.v.y = -player.speed;
                //console.log("jump");
            }
            pressed['up'] = true;
            if(!pressed['holding']) {
                anim.stop();
                anim.textures = spritesheet.animations.jumpDino;
                anim.play();
            }
            break;

        case "Enter":
            //play();
            reg();
            break;
        case "ArrowDown": 
        case "s":
            jumpSound.play();
            if(player.circle.y!=player.defY) {
                player.v.y = player.speed*3;
            }
            pressed['down'] = true;
            //anim = new PIXI.AnimatedSprite(spritesheet.animations.duckDino);
            //anim.play();
            if(!pressed['holdingDown']) {
                anim.stop();
                anim.textures = spritesheet.animations.duckDino;
                anim.play();
                pressed['holdingDown'] = true;
            }
            break;
    }
}
//moves on keydown
function onkeyup(ev) {
    switch (ev.key) {
        case "ArrowUp": 
        case "Spacebar":
        //spacebar is " " apparently
        case " ":
        case "w":
            //player.v.y = pressed['down']?player.speed:0; 
            pressed['up'] = false;
            pressed['holding'] = false;
            break;
        case "ArrowDown": 
        case "s":
            //player.v.y = 0; 
            pressed['down'] = false;
            pressed['holdingDown'] = false;
            anim.stop();
            anim.textures = spritesheet.animations.dino;
            anim.play();
            break;
    }
}


//score

function scoreLoop() {
    // socket.emit("getScore", (res) => {
    //     player.score = res;
    // });
    scoreText.text = player.score.toString();
    //socket.emit("move");
    socket.send(JSON.stringify({"PostPos":[postNum, pos, anim.y]}));
    postNum++;
    //update map positions;
    if(started) {
        mapSprites[username].x = pos * ratio;
    }
    //socket.send(JSON.stringify("GetData"));

    //table.setData(tableData);
}
//death
function deathKey(ev) {
    switch(ev.key) {
        case "Enter":
            //document.location.reload();
            window.addEventListener("keydown", onkeydown);
            window.addEventListener("keyup", onkeyup);
            app.stage.removeChild(deathText);
            for(i of using){
                app.stage.removeChild(i);
            }
            startGame();
            break;
    }
}

//collision detection
//obstacle, terrain
function boxesIntersect(a, b) {
    if (devMode) {
        return false;
    }
    const ab = a.getBounds();
    const bb = b.getBounds();
    return ab.x + (ab.width*0.6) > bb.x && ab.x < bb.x + (bb.width*0.6) && ab.y + (ab.height*0.9) > bb.y && ab.y < bb.y + (bb.height*0.7);
}

function toggleDev() {
    devMode = !devMode;
}

//game starts and everything
player = new Player(0xfcf8ec, 10, {x:0, y:0});
window.addEventListener("keydown", onkeydown);
window.addEventListener("keyup", onkeyup);

function updateTable(){
    //console.log(tableData);
    //console.time('test');
    table.setData(tableData);
    //console.timeEnd('test');
}
function checkSocket(){
    
    if (socket.readyState == WebSocket.CLOSED || socket.readyState == WebSocket.CLOSING){
        let disconnectText = new PIXI.Text("Socket Disconnected - Reload page...", {fontFamily: 'Arial', fontSize: 24, fill: "black", align: 'right'});
        // socket = new WebSocket(host);
        // socket.send(JSON.stringify({"RegPlayer":username}));
        // //TODO:DO room thingies
        // socket.send(JSON.stringify("QuickPlay"));
        // socket.send(JSON.stringify("Ready"));
    }

}

function startGame(){
    mapSprites = [];
    dead = false;
    speedup = 1.5;
    pos = 0;
    //clouds = [];
    anim.play();
    //obstacles = [];
    thing=null;
    //floor = [];
    index = 0;
    using = [];
    started = false;
    floorDex = 0;
    pressed = {};
    pressed['holding']=false;
    pressed['holdingDown'] = false;
    postNum = 0;
    spriteY = 285;
    countingDown = false;
    
    gameLoop_interval = setInterval(gameLoop, 1000/60);
    cloudLoop_interval = setInterval(cloudLoop, 1000/30);
    scoreLoop_interval = setInterval(scoreLoop, 100);
    //table_interval = setInterval(updateTable, 100);
    
    //socket.send(JSON.stringify("Ready"));
}
socketRenew_interval = setInterval(checkSocket, 5000);
//startGame();
