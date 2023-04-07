class obstacle{
    x;
    y;
    height;
    width;
    sprite;
    constructor(x, y, height, width, sprite) {
        //xy = lower left corner of obstacle
        this.x = x;
        this.y = y;
        this.height = height;
        this.width = width;
        this.sprite = sprite;
    }

    move(distance){
        this.x+=distance;
        this.sprite.x = this.x;

    }
    get x(){
        return this.x;
    }

}