const host = location.hostname;
const score_socket = io(host);

function scoreTable() {
    socket.emit("getAllScores", (res) => {
        console.log(res);
    });
}

setInterval(scoreTable, 1000);