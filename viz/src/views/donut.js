window.createDonutChart = (data) => {
  const donut_filter = (rarity) => (ev) => ev.item.rarity === rarity;

  const yellow_filter = (a) =>
    a.item &&
    a.item.weapon_type &&
    (a.item.weapon_type.includes("Knife") ||
      a.item.weapon_type.includes("Bayonet") ||
      a.item.weapon_type.includes("Daggers") ||
      a.item.weapon_type.includes("Karambit") ||
      a.item.weapon_type.includes("Gloves") ||
      a.item.weapon_type.includes("Wraps"));
  const not_yellow_filter = (a) => !yellow_filter(a);

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
          data.filter(donut_filter(6)).filter(not_yellow_filter).length,
          data.filter(donut_filter(6)).filter(yellow_filter).length,
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
