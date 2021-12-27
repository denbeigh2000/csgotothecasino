window.createDonutChart = (data) => {
  const donut_filter = (rarity) => (ev) => ev.item.rarity === rarity;

  const data_donut = {
    datasets: [
      {
        label: "Dataset 1",
        data: [
          data.filter(donut_filter(1)).length,
          data.filter(donut_filter(2)).length,
          data.filter(donut_filter(3)).length,
          data.filter(donut_filter(4)).length,
          data.filter(donut_filter(5)).length,
          data.filter(donut_filter(6)).length,
          data.filter(donut_filter(7)).length,
        ],
        backgroundColor: [
          "#AFAFAF",
          "#6496E1",
          "#4B69CD",
          "#8847FF",
          "#D32CE6",
          "#EB4B4B",
          "#CAAB05",
        ],
      },
    ],
  };

  const config = {
    type: "doughnut",
    options: {
      animation: {
        duration: 1000,
        enabled: true,
        responsive: true,
        loop: false,
      },
    },
    data: data_donut,
  };

  const update = (chart, event) => {
    data_donut.datasets[0].data[event.item.rarity - 1]++;
    chart.update();
  };

  return { config, update };
};
