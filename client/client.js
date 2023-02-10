const container = document.getElementById("container");
var image = new Image();
image.crossOrigin = "anonymous";
image.src = "https://dino-chrome.com/static/images/dino.jpg"

window.onload = function (){
    let app = new PIXI.Application({width: 1106});
    container.appendChild(app.view);
    let player = PIXI.Sprite.from(image);
    player.anchor.set(0.5);
    player.x = app.view.width / 2;
    player.y = app.view.height / 2;
    app.stage.addChild(player);
}