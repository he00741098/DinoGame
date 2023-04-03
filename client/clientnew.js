//create global coordinate system
const host = location.hostname;
const container = document.getElementById("container");
const socket = io(host);
const deathSound = new Audio("./sounds/hit.mp3");
const jumpSound = new Audio("./sounds/press.mp3");
const scoreSound = new Audio("./sounds/reached.mp3");

//TODO: OBJECT POOL!!!!!
let using = [];

//width and height
let w = 512, h = 512;
//images
const image = new Image();
image.src = '/images/DinoSprites.png';

//images
const atlasData = {
    frames: {
        Dino1: {
            frame: { x: 0, y: 0, w: 60, h: 60 },
            sourceSize: { w: 32, h: 32 },
            spriteSourceSize: { x: 0, y: 0, w: 32, h: 32 }
        },
        Dino2: {
            frame: { x: 60, y: 0, w: 60, h: 60 },
            sourceSize: { w: 32, h: 32 },
            spriteSourceSize: { x: 0, y: 0, w: 32, h: 32 }
        },
        DuckDino1: {
            frame: { x: 240, y: 0, w: 60, h: 60 },
            sourceSize: { w: 32, h: 32 },
            spriteSourceSize: { x: 0, y: 0, w: 32, h: 32 }
        },
        DuckDino2: {
            frame: { x: 0, y: 60, w: 60, h: 60 },
            sourceSize: { w: 32, h: 32 },
            spriteSourceSize: { x: 0, y: 0, w: 32, h: 32 }

        },
        JumpDino1: {
            frame: { x: 120, y: 0, w: 60, h: 60 },
            sourceSize: { w: 32, h: 32 },
            spriteSourceSize: { x: 0, y: 0, w: 32, h: 32 }
        },
        JumpDino2: {
            frame: { x: 180, y: 0, w: 60, h: 60 },
            sourceSize: { w: 32, h: 32 },
            spriteSourceSize: { x: 0, y: 0, w: 32, h: 32 }
        },
        cactus: {
            frame: { x: 0, y: 180, w: 60, h: 60 },
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
        dino: ['Dino1', 'Dino2'], //array of frames by name
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

//pixels/sec
let global_speed_unit = 1.5;
let playerXglobal = 0;
//the player sprite
let anim = new PIXI.AnimatedSprite(spritesheet.animations.dino);
anim.animationSpeed = 0.1666;
anim.play();


//rendering the app
let app = new PIXI.Application({ width: 1106, height: 310 });
app.renderer.backgroundColor = 0xffffff;

//score
let scoreText = new PIXI.Text("0", { fontFamily: 'Arial', fontSize: 24, fill: "black", align: 'right' });

//obstacles
//TODO: get obstacles from server
const obstacles = generateTerrain(10000);
let seed = [];
socket.emit("getObstacles", (res) => {
    seed = res.split(" ");
});

let obstacleIndex = 0;
function nextObstacle() {
    let ob = seed[obstacleIndex];
    obstacleIndex++;
    let sprite = new PIXI.AnimatedSprite(spritesheet.animations.cactus);
    return new obstacle(ob[0], 0, 0, 0, sprite);
}

//register player
socket.emit("registerPlayer", new URLSearchParams(document.location.search).get("username"), (res) => {
    switch (res) {
        case "ETAKEN":
            document.location.href = "/";
            break;
        default:
            break;
    }
});

//start the app
window.onload = function () {
    container.appendChild(app.view);
    anim.anchor.set(0.5);
    anim.x = app.view.width / 6;
    anim.y = app.view.height / 2;
    app.stage.addChild(anim);
    app.stage.addChild(scoreText);
}

let started = false;

//game loop function
//Updates player, increments position, moves obstacles
function gameLoop() {

    player.update();
    //checks if obstacles should be moved I think
    if (!started || nextObstacle().sprite.x < 0) {
        started = true;
        obstacles[index].sprite.width = 60;
        obstacles[index].sprite.height = 75;
        obstacles[index].x = app.view.width;
        obstacles[index].sprite.x = app.view.width;
        app.stage.addChild(obstacles[index].sprite);
        using.push(obstacles[index]);
        if (index < obstacles.length - 1) {
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


//game starts and everything
player = new Player(0xfcf8ec, 10, {x:0, y:0});
window.addEventListener("keydown", onkeydown);
window.addEventListener("keyup", onkeyup);
let fps = 60;
let gameLoop_interval = setInterval(gameLoop, 1000/fps);
let scoreLoop_interval = setInterval(scoreLoop, 100);