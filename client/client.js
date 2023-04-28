const host = location.hostname;
const container = document.getElementById("container");
const socket = io(host);
const deathSound = new Audio("./sounds/hit.mp3");
const jumpSound = new Audio("./sounds/press.mp3");
const scoreSound = new Audio("./sounds/reached.mp3");
const username = new URLSearchParams(document.location.search).get("username");
var devMode = false;
var mapSprites = [];

//width and height
let w = 512, h=512;
var ratio;
//images
const image = new Image();
image.src = '/images/DinoSprites.png';
const floorImage = new Image();
floorImage.src = '/images/floor.png';
//images
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
        cactus: {
            frame: {x: 0, y:180, w:60, h:60},
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
        floor22:['floor23'],
        floor24:['floor24'],
        floor25:['floor25']
	}
}

//more images
const spritesheet = new PIXI.Spritesheet(
	PIXI.BaseTexture.from(atlasData.meta.image),
	atlasData
);

const floorsheet = new PIXI.Spritesheet(
    PIXI.BaseTexture.from(floorData.meta.image),
    floorData
);
//parsing
spritesheet.parse();
floorsheet.parse();
//speed and position variables
let speedup = 1.5;
let pos = 0;

//clouds
let clouds = [];

//the player
let anim = new PIXI.AnimatedSprite(spritesheet.animations.dino);
anim.animationSpeed = 0.1666;
anim.play();

//rendering the app
let app = new PIXI.Application({width: 1106, height: 310});
app.renderer.backgroundColor = 0xffffff;
app.stage.sortableChildren = true;
//score
let scoreText = new PIXI.Text("0", {fontFamily: 'Arial', fontSize: 24, fill: "black", align: 'right'});
//obstacles
//TODO: get obstacles from server
const obstacles = generateTerrain();
//register player
socket.emit("registerPlayer", new URLSearchParams(document.location.search).get("username"), (res) => {
    switch(res) {
        case "ETAKEN":
            document.location.href = "/";
            break;
        default:
            break;
    }
});

//start the app
let floor = [];
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
let index = 0;
//TODO: object pool?
let using = [];
let started = false;
//game loop function
//Updates player, increments position, moves obstacles
let spriteY = 285;

function cloudLoop() {
    for(let c of clouds) {
        c.x-=(4*speedup);
        if(c.x<-100) {
            c.x=1106;
        }
    }
}

function gameLoop() {
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
let floorDex = 0;
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
                clearInterval(gameLoop_interval);
                clearInterval(scoreLoop_interval);
                clearInterval(cloudLoop_interval);
                window.removeEventListener("keydown", onkeydown);
                window.removeEventListener("keyup", onkeyup);
                anim.stop();
                socket.disconnect();
                let deathText = new PIXI.Text("You died! -- Press ENTER to retry", {fontFamily: 'Arial', fontSize: 24, fill: "black", align: 'right'});
                deathText.anchor.x = -1;
                deathText.anchor.y = -5;
                app.stage.addChild(deathText);
                window.addEventListener("keydown", deathKey);
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
let pressed = {};
pressed['holding']=false;
pressed['holdingDown'] = false;
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
    socket.emit("getScore", (res) => {
        player.score = res;
    });
    scoreText.text = player.score.toString();
    socket.emit("move");
    socket.emit("sendPos", pos, () =>{});
    //update map positions;
    if(started) {
        mapSprites[username].x = pos * ratio;
    }
    socket.emit("getAllPos", (result) => {
        let resi = JSON.parse(result);
        //console.log(resi);

        for(let res of resi){
            if(res['username']==username) {
                continue;
            }
        if(mapSprites[res['username']]!=null) {
            mapSprites[res['username']].x=res['pos']*ratio;
        }else{
            mapSprites[res['username']]= new PIXI.AnimatedSprite(spritesheet.animations.dino);
            mapSprites[res['username']].x=res['pos']*ratio;
            mapSprites[res['username']].y = spriteY;
            mapSprites[res['username']].width = 16;
            mapSprites[res['username']].height = 16;
            mapSprites[res['username']].tint = "#d9ad4e";
            app.stage.addChild(mapSprites[res['username']]);
        }
        }

    });
}
//death
function deathKey(ev) {
    switch(ev.key) {
        case "Enter":
            document.location.reload();
            break;
    }
}
//Generate terrain - WILL BE REMOVED
var thing;
function generateTerrain(){
    let obstacles = [];
    socket.emit("getObstacles", (res)=>{

        thing = JSON.parse(res);
        //console.log(thing);
        for(const i of thing){
            //TODO: add different types of obstacles and also object pool
            let sprite = new PIXI.AnimatedSprite(spritesheet.animations.cactus);
            let defY = (h/4);
            sprite.x = app.view.width;
            sprite.y = defY-sprite.height;
            obstacles.push(new obstacle(i['x'], i['y'], 2, 5, sprite));
            //console.log("added obstacle: " + i['x'] + " " + i['y']);
        }

    });
    return obstacles;
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
let gameLoop_interval = setInterval(gameLoop, 1000/60);
let cloudLoop_interval = setInterval(cloudLoop, 1000/30);
let scoreLoop_interval = setInterval(scoreLoop, 100);
