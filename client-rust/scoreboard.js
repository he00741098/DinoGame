const score_socket = io(location.hostname);

var table = new Tabulator("#score-table", {
    height:"300px",
    layout:"fitDataTable",
    initialSort:[{column:"score", dir:"desc"}],
    columns:[{title:"Username", field:"username"},{title:"Score", field:"score", sorter: "number"}]
});

table.on("tableBuilt", () => {
    table.setPage(2);
});

function scoreTable() {
    score_socket.emit("getAllScores", (res) => {
        table.setData(res);
    });
}

setInterval(scoreTable, 100);
