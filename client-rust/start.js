const host = location.hostname;

function play() {
    let box = document.getElementById("usernamebox");
    if(box.value == "") {
        alert("please input a username");
        return;
    }
    let check_sock = new WebSocket("wss://rustdinogame.herokuapp.com/");
    check_sock.send(JSON.stringify({"RegPlayer":box.value}));
    
    check_sock.onopen = function (res) {
        switch(res) {
            case "NameTaken":
                alert("Username already taken, try another one");
                break;
            case "RegPlayer":
                document.location.href = `${document.location.href}/game.html?username=${box.value}`;
                break;
            default:
                break;
        }
    }
}

function playEnter(ev) {
    if(ev.key == "Enter") {
        play();
    }
}

window.addEventListener("keydown", playEnter);