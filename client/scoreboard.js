const score_socket = io("ws://127.0.0.1:6969");

function scoreTable() {
    socket.emit("getAllScores", (res) => {
        console.log(res);
    });
}

setInterval(scoreTable, 1000);