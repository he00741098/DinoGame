const host = location.hostname;

function play() {
    let box = document.getElementById("usernamebox");
    let box2 = document.getElementById("roombox");
    if(box.value == "") {
        alert("please input a username");
        return;
    }
    if(box2.value == "") {
        box2.value = "global";
    }
    let check_sock = io(host);
    check_sock.emit("checkPlayerStatus", box.value, box2.value, (res) => {
        switch(res) {
            case "EBADREQ":
                alert("Bad request");
                break;
            case "TAKEN":
                alert("Username or roomid already taken/started, try another one");
                break;
            case "EALREADYSTARTED":
                alert("Room already started");
                break;
            case "NOT TAKEN":
                //document.location.href = `${document.location.href}/game.html?username=${box.value}&room=${box2.value}`;
                const url = new URL(document.location);
                url.searchParams.append("username", box.value);
                url.searchParams.append("room", box2.value);
                window.history.pushState({}, "", url);
                document.getElementsByClassName("center")[0].style.display = "none";
                document.getElementsByClassName("game")[0].style.display = "block";
                window.removeEventListener("keydown", playEnter);

                var client = document.createElement('script');
                client.src = "./client.js";
                document.head.appendChild(client);

                break;
            default:
                break;
        }
    });
}

function playEnter(ev) {
    if(ev.key == "Enter") {
        play();
    }
}

window.addEventListener("keydown", playEnter);