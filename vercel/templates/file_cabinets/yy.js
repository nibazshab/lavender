fetch("https://yyapi.xpdbk.com/api/ian?type=text")
    .then(response => response.text())
    .then(data => {
        document.getElementById("yy").textContent = data || "加载失败";
    })
    .catch(() => {
        document.getElementById("yy").textContent = "一言加载失败";
    });
