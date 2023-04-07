const host = location.hostname;
const container = document.getElementById("container");
const socket = io(host);
const deathSound = new Audio("./sounds/hit.mp3");
const jumpSound = new Audio("./sounds/press.mp3");
const scoreSound = new Audio("./sounds/reached.mp3");

//width and height
let w = 512, h=512;
//images
const image = new Image();
image.src = '/images/DinoSprites.png';

let global_player_x = 0;


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
const obstacles = generateTerrain(10000);
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
    if(!started||obstacles[index-1].sprite.x<0){
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
                console.log("jump");
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
function generateTerrain(length){

    socket.emit("getObstacles", (res)=>{
        for(const i in res){
            console.log(i);
        }
    });

    let obstacle_distance = 100;
    let obstacles = [];
    for(i=0; i<length; i+=obstacle_distance){
        let sprite = new PIXI.AnimatedSprite(spritesheet.animations.cactus);
        let defY = (h/4);
        sprite.x = app.view.width;
        sprite.y = defY-sprite.height;
        //sprite.height = sprite.height*0.25;
        //sprite.width = sprite.width*0.25;
        obstacles.push(new obstacle(i, 0, 5, 2, sprite));
        //console.log("added new obstacle at "+i+","+0);
    }
    return obstacles;
}
//collision detection
function boxesIntersect(a, b) {
    var ab = a.getBounds();
    var bb = b.getBounds();
    return ab.x + (ab.width*0.6) > bb.x && ab.x < bb.x + (bb.width*0.6) && ab.y + (ab.height*0.7) > bb.y && ab.y < bb.y + (bb.height*0.7);
}

//game starts and everything
player = new Player(0xfcf8ec, 10, {x:0, y:0});
window.addEventListener("keydown", onkeydown);
window.addEventListener("keyup", onkeyup);
let gameLoop_interval = setInterval(gameLoop, 1000/60);
let scoreLoop_interval = setInterval(scoreLoop, 100);
