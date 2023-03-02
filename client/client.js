//import * as TWEEDLE from TWEEDLE;
const container = document.getElementById("container");
const socket = io("ws://127.0.0.1:6969");
const deathSound = new Audio("./sounds/hit.mp3");
const jumpSound = new Audio("./sounds/press.mp3");
const scoreSound = new Audio("./sounds/reached.mp3");

const image = new Image();
image.src = '/images/DinoSprites.png';

let pos = 0;
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

const spritesheet = new PIXI.Spritesheet(
	PIXI.BaseTexture.from(atlasData.meta.image),
	atlasData
);

spritesheet.parse();

let anim = new PIXI.AnimatedSprite(spritesheet.animations.dino);
anim.animationSpeed = 0.1666;
anim.play();
let app = new PIXI.Application({width: 1106, height: 310});
app.renderer.backgroundColor = 0x456268;

// TODO: add score and center this
let scoreText = new PIXI.Text("0", {fontFamily: 'Arial', fontSize: 24, fill: "white", align: 'right'});
//scoreText.anchor.set(0.5, 0.5);
//scoreText.position.set(750,100);
const obstacles = generateTerrain(1000);
window.onload = function (){
    //let app = new PIXI.Application({width: 1106});
    container.appendChild(app.view);
    anim.anchor.set(0.5);
    anim.x = app.view.width / 6;
    anim.y = app.view.height / 2;
    app.stage.addChild(anim);
    app.stage.addChild(scoreText);
}
let index = 0;
let using = [];
function gameLoop() {
    
    player.update();
    pos++;
    if(obstacles[index].x<pos+10){
        //console.log("I ran 100");
        index++;
        app.stage.addChild(obstacles[index].sprite);
        using.push(obstacles[index]);
    }

    move(using);
    socket.emit("getAllScores");
}

function move(obstacle_list){
for(i = 0; i<obstacle_list.length; i++){
if(obstacle_list[i].x<pos){
    //console.log("deleting stuff");
    
    //obstacle_list[i].destroy();
    obstacle_list.splice(i,obstacle_list.length-1);
    obstacles.splice(i,obstacle_list.length-1);
}else{
    obstacle_list[i].x--;
    obstacle_list[i].sprite.x--;
}

}


}

let w = 512, h=512;
let pressed = {};
pressed['holding']=false;
pressed['holdingDown'] = false;
class Circle {
    constructor(color, radius, v) {
        this.radius = radius;
        this.v = v;

        let circle = anim;
        //circle.beginFill(color);
        //circle.drawCircle(0, 0, radius);
        //circle.endFill();
        //circle.x = radius;
        //circle.y = radius;
        //app.stage.addChild(circle);

        this.circle = circle;
    }
}


class Player extends Circle {
    constructor(color, radius, v) {
        super(color, radius, v);
        this.defX = app.view.width / 6;
        this.defY = anim.y = app.view.height / 2;
        this.maxHeight = 25;
        this.score = 0;

        this.reset();
    }

    reset() {
        this.circle.x = w/2;
        this.circle.y = h/2;
        this.speed = 10;
    }

    update() {
        
        if(this.circle.y<this.maxHeight){
            //pressed['up'] = false;
            this.v.y=this.speed;
            console.log("too high");
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
            console.log("too low");
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

function setupControls() {
    window.addEventListener("keydown", onkeydown);
    window.addEventListener("keyup", onkeyup);
}

//socket.on("disconnect", (reason) => {
//    document.body.innerHTML = reason;
//});

function scoreLoop() {
    socket.emit("getScore", (res) => {
        player.score = res;
    });
    scoreText.text = player.score.toString();
    socket.emit("move");
}



function generateTerrain(length){

let obstacle_distance = 10;
let obstacles = [];
for(i=0; i<length; i+=obstacle_distance){
    obstacles.push(new obstacle(i, 0, 5, 2, new PIXI.AnimatedSprite(spritesheet.animations.cactus)));
    console.log("added new obstacle at "+i+","+0);
}


return obstacles;
}




//---
player = new Player(0xfcf8ec, 10, {x:0, y:0});
setupControls();
setInterval(gameLoop, 1000/60);
setInterval(scoreLoop, 100);