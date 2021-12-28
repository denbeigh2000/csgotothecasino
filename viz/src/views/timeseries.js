window.createTimeseriesChart = (data) => {
  document.body.style.backgroundColor = "#222";
  groups = new Set();
  const grouped = {};
  data.forEach((d) => groups.add(d.name));
  for (k of groups.keys()) {
    grouped[k] = zip(
      data.map((d) => new Date(d.at)),
      sum_over_time(data.filter((d) => d.name === k).map(value_estimator))
    ).map(([date, n]) => ({ x: date, y: n }));
  }

  console.log(grouped);

  const animation = {
    x: {
      easing: "easeOutQuart",
      duration: (ctx) => {
        if (ctx.type === "data") {
          if (!ctx.raw.dropped_x_duration) {
            ctx.raw.dropped_x_duration = true;
            return 4000;
          }
        }
        return 0;
      },
      from: (ctx) => {
        if (ctx.type === "data") {
          if (!ctx.raw.dropped_x) {
            ctx.raw.dropped_x = true;
            return ctx.raw.last
              ? ctx.chart.scales.x.getPixelForValue(ctx.raw.last.x)
              : 0;
          }
        }
      },
    },
    y: {
      easing: "easeOutQuart",
      duration: (ctx) => {
        if (ctx.type === "data") {
          if (!ctx.raw.dropped_y_duration) {
            ctx.raw.dropped_y_duration = true;
            return 4000;
          }
        }
        return 0;
      },
      from: (ctx) => {
        if (ctx.type === "data") {
          if (!ctx.raw.dropped_y) {
            ctx.raw.dropped_y = true;
            return ctx.raw.last
              ? ctx.chart.scales.y.getPixelForValue(ctx.raw.last.y)
              : 0;
          }
        }
      },
    },
  };

  const chart_data = {
    datasets: Object.entries(grouped).map(([group, group_data]) => ({
      label: group,
      data: group_data,
      ...getPlayerDefaults(group),
    })),
  };
  const config = {
    type: "line",
    data: chart_data,
    options: {
      animations: animation,
      events: [],
      plugins: {
        legend: {
          labels: {
            boxWidth: 10,
            boxHeight: 10,
            usePointStyle: true,
            font: {
              size: 30,
            },
          },
        },
        zoom: {
          pan: {
            enabled: true,
            mode: "x",
          },
          zoom: {
            wheel: {
              enabled: true,
            },
            mode: "x",
          },
          transitions: {
            zoom: {
              animation: {
                duration: 0,
              },
            },
          },
          limits: {
            x: {
              minDelay: 0,
              maxDelay: 0,
              minDuration: 400000,
              maxDuration: 14400000,
            },
          },
        },
      },
      scales: {
        x: {
          ticks: {
            font: {
              size: 24,
            },
          },
          grid: {
            lineWidth: 2,
          },
          type: "realtime",
          realtime: {
            duration: 400000,
            delay: 0,
          },
        },
        y: {
          suggestedMin: -40,
          suggestedMax: 40,
          grid: {
            lineWidth: 2,
            color: (ctx) => {
              if (ctx.tick.value === 0) {
                return "#602020";
              }
              return "#1e1e1e";
            },
          },
          ticks: {
            font: {
              size: 24,
            },
            // Include a dollar sign in the ticks
            callback: function (value, index, ticks) {
              // call the default formatter, forwarding `this`
              return (
                "$" +
                Chart.Ticks.formatters.numeric.apply(this, [
                  value,
                  index,
                  ticks,
                ])
              );
            },
          },
        },
      },
    },
  };

  // Callback function to update the chart when events arrive via the websocket.
  const update = (chart, event) => {
    let found = false;
    for (let i = 0; i < chart_data.datasets.length; ++i) {
      if (chart_data.datasets[i].label === event.name) {
        found = true;
        const a = chart_data.datasets[i].data;
        const { last, ...last_val } = a[a.length - 1];
        const datum = {
          x: new Date(event.at),
          y: value_estimator(event) + last_val.y,
          last: last_val,
        };
        chart_data.datasets[i].data.push(datum);
      }
    }
    if (!found) {
      const datum = {
        x: new Date(event.at),
        y: value_estimator(event),
      };
      chart_data.datasets.push({
        label: event.name,
        data: [datum],
        ...getPlayerDefaults(event.name),
      });
    }
    chart.update();
  };
  return { config, update };
};
