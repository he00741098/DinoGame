//import * as TWEEDLE from TWEEDLE;
const container = document.getElementById("container");

const image = new Image();
image.src = '/images/DinoSprites.png';

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
	},
	meta: {
		image: image,
		format: 'RGBA8888',
		size: { w: 128, h: 32 },
		scale: 1
	},
	animations: {
		dino: ['Dino1','Dino2'] //array of frames by name
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
let app = new PIXI.Application({width: 1106, height: 250});
app.renderer.backgroundColor = 0x456268;

window.onload = function (){
    //let app = new PIXI.Application({width: 1106});
    container.appendChild(app.view);
    anim.anchor.set(0.5);
    anim.x = app.view.width / 6;
    anim.y = app.view.height / 2;
    app.stage.addChild(anim);


}

function gameLoop() {
    player.update();

}

let w = 512, h=512;
let pressed = {};
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
        }
        if(this.circle.y>this.defY){
            //pressed['down'] = false;
            this.v.y=0;
            this.circle.y = this.defY;
            console.log("too low");
        }
        //console.log(this.circle.y);
        let x = this.circle.x + this.v.x;
        let y = this.circle.y + this.v.y;

        this.circle.x = Math.min(Math.max(x, this.radius), w-this.radius);
        this.circle.y = Math.min(Math.max(y, this.radius), w-this.radius);

    

    }
}

function onkeydown(ev) {
    switch (ev.key) {


        case "ArrowUp":
        case "Spacebar":
        //spacebar is " " apparently
        case " ":
        case "w":
            if(player.circle.y==player.defY){
                player.v.y = -player.speed;
                console.log("jump");
            }
            pressed['up'] = true;
            break;

        case "ArrowDown": 
        case "s":
            if(player.circle.y!=player.defY){
            player.v.y = player.speed*3;
            }
            pressed['down'] = true;

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
            break;

        case "ArrowDown": 
        case "s":
            //player.v.y = 0; 
            pressed['down'] = false;
            break;
    }
}

function setupControls() {
    window.addEventListener("keydown", onkeydown);
    window.addEventListener("keyup", onkeyup);
}

//---
player = new Player(0xfcf8ec, 10, {x:0, y:0});
setupControls();
setInterval(gameLoop, 1000/60);