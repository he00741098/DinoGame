const score_socket = io(host);

function scoreTable() {
    score_socket.emit("getAllScores", (res) => {
        console.log(res);
    });
}

setInterval(scoreTable, 1000);
