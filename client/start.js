const host = 'https://multiplayerdino.herokuapp.com/';

function play() {
    let box = document.getElementById("usernamebox");
    if(box.value == "") {
        alert("please input a username");
        return;
    }
    let check_sock = io(host);
    check_sock.emit("checkPlayerUsername", box.value, (res) => {
        switch(res) {
            case "EBADREQ":
                alert("Bad request");
                break;
            case "TAKEN":
                alert("Username already taken, try another one");
                break;
            case "NOT TAKEN":
                document.location.href = `${document.location.href}/game.html?username=${box.value}`;
                break;
            default:
                break;
        }
    });
}