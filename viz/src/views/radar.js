window.createRadarChart = (data) => {
  document.body.style.backgroundColor = "#222";
  const wear_filter = (wear_name) => (ev) => ev.item.wear_name === wear_name;

  const player_filter = (player_name) => (ev) => ev.name === player_name;

  const data_radar = {
    labels: [
      "Battle-Scarred",
      "Well-Worn",
      "Field-Tested",
      "Minimal Wear",
      "Factory New",
    ],
    datasets: [],
  };

  const groups = new Set();
  Object.values(Players).forEach((d) => groups.add(d));

  for (group of groups.keys()) {
    const grouped = data.filter(player_filter(group));
    data_radar.datasets.push({
      label: group,
      data: [
        grouped.filter(wear_filter("Battle-Scarred")).length,
        grouped.filter(wear_filter("Well-Worn")).length,
        grouped.filter(wear_filter("Field-Tested")).length,
        grouped.filter(wear_filter("Minimal Wear")).length,
        grouped.filter(wear_filter("Factory New")).length,
      ],
      borderColor: PlayerColors[group] || "red",
      backgroundColor: (PlayerColors[group] || "#ff0000") + "50",
    });
  }
  const config = {
    type: "radar",
    options: {
      animation: {
        duration: 2000,
        enabled: true,
        responsive: true,
        loop: false,
      },
      responsive: true,
      plugins: {
        legend: {
          display: false,
        },
      },
      scales: {
        r: {
          ticks: {
            display: false,
          },
          grid: {
            color: "#444",
            lineWidth: 2,
          },
          angleLines: {
            color: "#444",
            lineWidth: 2,
          },
          pointLabels: {
            font: {
              size: 24,
            },
            color: "#999",
          },
        },
      },
    },
    data: data_radar,
  };

  const update = (chart, event) => {
    const id = (Object.values(Players).indexOf(event.name));
    data_radar.datasets[id].data[event.item.rarity - 1]++;
    chart.update();
  };

  return { config, update };
};
