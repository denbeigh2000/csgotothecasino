window.createTimeseriesChart = (data) => {
  groups = new Set();
  const grouped = {};
  data.forEach((d) => groups.add(d.name));
  for (k of groups.keys()) {
    grouped[k] = zip(
      data.map((d) => new Date(d.at)),
      sum_over_time(data.filter((d) => d.name === k).map(value_estimator))
    ).map(([date, n]) => ({ x: date, y: n }));
  }

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
      cubicInterpolationMode: "monotone",
      tension: 0.4,
      ...getPlayerColors(group),
    })),
  };
  const config = {
    type: "line",
    data: chart_data,
    options: {
      animations: animation,
      interaction: {
        mode: "nearest",
        axis: "x",
        intersect: false,
      },
      plugins: {
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
          type: "realtime",
          realtime: {
            duration: 400000,
            delay: 0,
          },
        },
        y: {
          suggestedMin: -100,
          suggestedMax: 100,
          ticks: {
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
    for (let i = 0; i < chart_data.datasets.length; ++i) {
      if (chart_data.datasets[i].label === event.name) {
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
    chart.update();
  };
  return { config, update };
};
