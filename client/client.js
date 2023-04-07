const host = location.hostname;
const container = document.getElementById("container");
const socket = io(host);
const deathSound = new Audio("./sounds/hit.mp3");
const jumpSound = new Audio("./sounds/press.mp3");
const scoreSound = new Audio("./sounds/reached.mp3");
var mapSprites = [];

//width and height
let w = 512, h=512;
var ratio;
//images
const image = new Image();
image.src = '/images/DinoSprites.png';
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
        cactus: ['cactus']
	}
}
//more images
const spritesheet = new PIXI.Spritesheet(
	PIXI.BaseTexture.from(atlasData.meta.image),
	atlasData
);
//parsing
spritesheet.parse();
//speed and position variables
let speedup = 1.5;
let pos = 0;


//the player
let anim = new PIXI.AnimatedSprite(spritesheet.animations.dino);
anim.animationSpeed = 0.1666;
anim.play();

//rendering the app
let app = new PIXI.Application({width: 1106, height: 310});
app.renderer.backgroundColor = 0xffffff;
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
window.onload = function (){
    container.appendChild(app.view);
    anim.anchor.set(0.5);
    anim.x = app.view.width / 6;
    anim.y = app.view.height / 2;
    app.stage.addChild(anim);
    app.stage.addChild(scoreText);
    //put dino on "map"
    //TODO: look at app ticker


}

//more variables for game
let index = 0;
//TODO: object pool?
let using = [];
let started = false;
//game loop function
//Updates player, increments position, moves obstacles

function gameLoop() {
    
    player.update();
    //pos++;
    //checks if obstacles should be moved I think
    if(obstacles.length>0&&(!started||obstacles[index].x<pos-anim.x+1106)){
        if(!started){
            //TODO: show a map of all the player positions, also make a width variable
            ratio = 1106/obstacles[obstacles.length-1]["x"];
            console.log("the ration:" + ratio);
            let mapSprite = new PIXI.AnimatedSprite(spritesheet.animations.dino);
            //mapSprite.height= 60;
            //mapSprite.width= 30;
            mapSprite.x = pos*ratio;
            console.log("mapSpriteX:"+mapSprite.x);
            mapSprite.y = 285;
            app.stage.addChild(mapSprite)
            mapSprites.push({name: "main",sprite:mapSprite});
            console.log("added map sprite");
        }else{
            mapSprites[0].sprite.x = pos*ratio;
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
function move(obstacle_list){
    for(i = obstacle_list.length-1; i>=0; i--){
        if(obstacle_list[i].sprite.x<-220){
            using = using.splice(0, i).concat(using.splice(i+1));
            obstacle_list = using;
        } else {
            //TODO: Slow down player instead of death in multiplayer game
            //TODO: Fix death when player reaches end of map. Instead show "you win" or something
            //console.log(obstacle_list[i].sprite);
            if(boxesIntersect(obstacle_list[i].sprite, anim)){
                clearInterval(gameLoop_interval);
                clearInterval(scoreLoop_interval);
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
        this.defY = anim.y = app.view.height / 2;
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
function boxesIntersect(a, b) {
    const ab = a.getBounds();
    const bb = b.getBounds();
    return ab.x + (ab.width*0.6) > bb.x && ab.x < bb.x + (bb.width*0.6) && ab.y + (ab.height*0.7) > bb.y && ab.y < bb.y + (bb.height*0.7);
}

//game starts and everything
player = new Player(0xfcf8ec, 10, {x:0, y:0});
window.addEventListener("keydown", onkeydown);
window.addEventListener("keyup", onkeyup);
let gameLoop_interval = setInterval(gameLoop, 1000/60);
let scoreLoop_interval = setInterval(scoreLoop, 100);
