// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded "><a href="index.html"><strong aria-hidden="true">1.</strong> Overview</a></li><li class="chapter-item expanded "><a href="thesis/overview.html"><strong aria-hidden="true">2.</strong> Thesis Overview</a></li><li class="chapter-item expanded "><a href="thesis/Ekeland-Hofer-Zehnder-Capacity.html"><strong aria-hidden="true">3.</strong> EHZ Capacity</a></li><li class="chapter-item expanded "><a href="thesis/capacity-algorithm-oriented-edge-graph.html"><strong aria-hidden="true">4.</strong> Oriented-Edge Graph Algorithm</a></li><li class="chapter-item expanded "><a href="thesis/capacity-algorithm-linear-program.html"><strong aria-hidden="true">5.</strong> LP/QP Programs for c_EHZ</a></li><li class="chapter-item expanded "><a href="thesis/visualization.html"><strong aria-hidden="true">6.</strong> Visualization &amp; Verification</a></li><li class="chapter-item expanded "><a href="meta/overview.html"><strong aria-hidden="true">7.</strong> Meta</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="meta/benchmarks.html"><strong aria-hidden="true">7.1.</strong> Benchmarks → Docs</a></li><li class="chapter-item expanded "><a href="meta/oriented-edge-charts-and-rotation.html"><strong aria-hidden="true">7.2.</strong> OE: Charts and Rotation</a></li><li class="chapter-item expanded "><a href="meta/git-history-cleanup.html"><strong aria-hidden="true">7.3.</strong> Git History Cleanup</a></li><li class="chapter-item expanded "><a href="meta/protocol-2025-11-04.html"><strong aria-hidden="true">7.4.</strong> Protocol — 2025-11-04 Meeting</a></li></ol></li><li class="chapter-item expanded "><a href="thesis/geom2d_polytopes.html"><strong aria-hidden="true">8.</strong> 2D Polytopes (H-rep)</a></li><li class="chapter-item expanded "><a href="thesis/geom4d_polytopes.html"><strong aria-hidden="true">9.</strong> 4D Polytopes (H/V-rep)</a></li><li class="chapter-item expanded "><a href="thesis/geom4d_volume.html"><strong aria-hidden="true">10.</strong> 4D Volume Algorithm</a></li><li class="chapter-item expanded "><a href="thesis/random-polytopes.html"><strong aria-hidden="true">11.</strong> Random Polytope Generators</a></li><li class="chapter-item expanded "><a href="thesis/status-math.html"><strong aria-hidden="true">12.</strong> Implementation Status FAQ</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0].split("?")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
