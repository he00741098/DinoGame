const container = document.getElementById("container");

const atlasData = {
	frames: {
		Dino1: {
			frame: { x: 0, y:0, w:32, h:32 },
			sourceSize: { w: 32, h: 32 },
			spriteSourceSize: { x: 0, y: 0, w: 32, h: 32 }
		},
		Dino2: {
			frame: { x: 32, y:0, w:32, h:32 },
			sourceSize: { w: 32, h: 32 },
			spriteSourceSize: { x: 0, y: 0, w: 32, h: 32 }
		},
	},
	meta: {
		image: 'images/spritesheet.png',
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

const anim = new PIXI.AnimatedSprite(spritesheet.animations.dino);
anim.animationSpeed = 0.1666;
anim.play();


//var image = new Image();



image.crossOrigin = "anonymous";
//image.src = "https://dino-chrome.com/static/images/dino.jpg"

window.onload = function (){
    let app = new PIXI.Application({width: 1106});
    container.appendChild(app.view);
    //let player = anim;
    player.anchor.set(0.5);
    player.x = app.view.width / 2;
    player.y = app.view.height / 2;
    app.stage.addChild(anim);
}