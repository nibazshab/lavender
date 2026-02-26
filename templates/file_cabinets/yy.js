fetch("https://yyapi.xpdbk.com/api/ian?type=text")
    .then(r => r.text())
    .then(data => {
        document.getElementById("yy").textContent = data;
    });