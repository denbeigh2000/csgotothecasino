window.createTableChart = (data) => {
  document.body.style.backgroundColor = "#222";
  // Callback function to update the chart when events arrive via the websocket.

  const init = () => {
    document.getElementById("main").innerHTML = "";
    data.reverse();
    data.forEach((row) => {
      document.getElementById("main").appendChild(makeRow(row));
    });
  };

  const update = (chart, event) => {
    document.getElementById("main").innerHTML = "";
    data.unshift(event);
    data.forEach((row) => {
      document.getElementById("main").appendChild(makeRow(row));
    });
  };
  return { init, update };
};
