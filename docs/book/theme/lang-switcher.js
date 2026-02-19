// Language switcher for Synaptic documentation
// Injects an EN/中文 toggle into the mdBook menu bar.
(function () {
  "use strict";

  var path = window.location.pathname;

  // Detect current language from URL path
  var isZh = /\/zh\//.test(path);
  var isEn = /\/en\//.test(path);

  // Build the target path for the other language
  function switchPath(from, to) {
    return path.replace("/" + from + "/", "/" + to + "/");
  }

  function insertSwitcher() {
    var rightButtons = document.querySelector(".right-buttons");
    if (!rightButtons) return;

    var container = document.createElement("div");
    container.className = "lang-switcher";

    if (isEn || isZh) {
      // Both languages in /en/ and /zh/ subdirectories
      var enLink = document.createElement("a");
      enLink.href = isEn ? "#" : switchPath("zh", "en");
      enLink.textContent = "EN";
      enLink.className = isEn ? "lang-active" : "";
      enLink.title = "English";
      if (isEn) enLink.onclick = function (e) { e.preventDefault(); };

      var sep = document.createElement("span");
      sep.className = "lang-sep";
      sep.textContent = "|";

      var zhLink = document.createElement("a");
      zhLink.href = isZh ? "#" : switchPath("en", "zh");
      zhLink.textContent = "\u4E2D\u6587";
      zhLink.className = isZh ? "lang-active" : "";
      zhLink.title = "\u5207\u6362\u5230\u4E2D\u6587";
      if (isZh) zhLink.onclick = function (e) { e.preventDefault(); };

      container.appendChild(enLink);
      container.appendChild(sep);
      container.appendChild(zhLink);
    } else {
      // Fallback: guess /zh/ is nested under current root
      var zhFallback = document.createElement("a");
      zhFallback.href = path.replace(/^(\/[^/]+\/)/, "$1zh/");
      zhFallback.textContent = "\u4E2D\u6587";
      zhFallback.title = "\u5207\u6362\u5230\u4E2D\u6587";
      container.appendChild(zhFallback);
    }

    rightButtons.insertBefore(container, rightButtons.firstChild);
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", insertSwitcher);
  } else {
    insertSwitcher();
  }
})();
