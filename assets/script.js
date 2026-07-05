document.querySelectorAll(".cmd-copy").forEach(function (button) {
  button.addEventListener("click", function () {
    var pre = button.closest(".cmd").querySelector("code");
    var text = pre ? pre.textContent : "";

    navigator.clipboard.writeText(text).then(function () {
      var original = button.textContent;
      button.textContent = "copied";
      button.classList.add("copied");
      setTimeout(function () {
        button.textContent = original;
        button.classList.remove("copied");
      }, 1600);
    });
  });
});
