window.createExpensiveChart = (data) => {
  document.body.style.backgroundColor = "#222";

  const mostExpensive = (arr) => {
    return arr
      .map((ev) => ({
        ...ev,
        price: value_estimator(ev),
      }))
      .sort(function (a, b) {
        return b.price - a.price;
      })[0];
  };

  const init = () => {
    document.getElementById("main").innerHTML = "";
    const most = mostExpensive(data);
    if (!most) return;
    document.getElementById("main").appendChild(makeRow(most));
  };

  const update = (chart, event) => {
    document.getElementById("main").innerHTML = "";
    data.push(event);
    const most = mostExpensive(data);
    if (!most) return;
    document.getElementById("main").appendChild(makeRow(most));
  };

  return { init, update };
};
