
const canvas = document.getElementById("game");
const canvas_context = canvas.getContext("2d");
canvas_context.fillStyle = "#ffffff";
canvas_context.textBaseline = 'middle';
canvas_context.textAlign = 'center';
canvas_context.font = "25px Arcade";
canvas_context.fillText("hello", canvas.width / 2, (canvas.height / 2) - 50);
