window.createExpensiveChart = (data) => {
  // Callback function to update the chart when events arrive via the websocket.
  const init = () => {
    JSON.stringify(
      (document.getElementById("main").innerText = data
        .map((ev) => ({
          ...ev,
          price: value_estimator(ev),
        }))
        .sort(function (a, b) {
          return a.price - b.price;
        })[0])
    );
  };
  const update = (chart, event) => {};
  return { init, update };
};
