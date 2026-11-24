// A real UIPageViewController with the same 5 pages as the `Pages` screen of the
// demo, to compare the engine `PageView` with it in the simulator, see
// `make sim args="reference"`.
//
// With HILEN_MEASURE=1 it prints every touch and every frame in which the
// pages moved, with the time, so a gesture of the sim helper can be measured
// on both:
//
//   touch <time> <phase> <x> <y>
//   offset <time> <x> <width> tracking|dragging|decelerating|idle
//   will <page>
//   did <page> <completed>

import UIKit

let pageColors: [UIColor] = [
    UIColor(red: 0.20, green: 0.47, blue: 0.96, alpha: 1),
    UIColor(red: 0.95, green: 0.45, blue: 0.20, alpha: 1),
    UIColor(red: 0.18, green: 0.70, blue: 0.42, alpha: 1),
    UIColor(red: 0.62, green: 0.33, blue: 0.85, alpha: 1),
    UIColor(red: 0.90, green: 0.25, blue: 0.40, alpha: 1),
]

let measuring = ProcessInfo.processInfo.environment["HILEN_MEASURE"] == "1"

func report(_ line: String) {
    if measuring {
        print(line)
    }
}

final class Page: UIViewController {
    let index: Int

    init(index: Int) {
        self.index = index
        super.init(nibName: nil, bundle: nil)
    }

    required init?(coder: NSCoder) {
        fatalError("no storyboard")
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = pageColors[index]
        let label = UILabel()
        label.text = "Page \(index + 1)"
        label.font = .systemFont(ofSize: 44, weight: .bold)
        label.textColor = .white
        label.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(label)
        NSLayoutConstraint.activate([
            label.centerXAnchor.constraint(equalTo: view.centerXAnchor),
            label.centerYAnchor.constraint(equalTo: view.centerYAnchor),
        ])
    }
}

final class Pages: UIViewController, UIPageViewControllerDataSource, UIPageViewControllerDelegate {
    let pager = UIPageViewController(transitionStyle: .scroll, navigationOrientation: .horizontal)
    var scroll: UIScrollView?
    var last = CGFloat.nan

    override func viewDidLoad() {
        super.viewDidLoad()
        addChild(pager)
        pager.view.frame = view.bounds
        pager.view.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        view.addSubview(pager.view)
        pager.didMove(toParent: self)
        pager.dataSource = self
        pager.delegate = self
        pager.setViewControllers([Page(index: 0)], direction: .forward, animated: false)
        scroll = pager.view.subviews.compactMap { $0 as? UIScrollView }.first
        if measuring {
            let link = CADisplayLink(target: self, selector: #selector(tick))
            link.add(to: .main, forMode: .common)
        }
    }

    // The scroll view of the pager keeps the open page 1 width from its left
    // end, so the offset is told against that.
    @objc func tick() {
        guard let scroll else { return }
        let x = scroll.contentOffset.x - scroll.bounds.width
        if x == last { return }
        last = x
        let state =
            scroll.isTracking ? "tracking"
            : scroll.isDragging ? "dragging" : scroll.isDecelerating ? "decelerating" : "idle"
        report(String(format: "offset %.4f %.2f %.0f %@", CACurrentMediaTime(), x, scroll.bounds.width, state))
    }

    func pageViewController(
        _ pageViewController: UIPageViewController, viewControllerBefore viewController: UIViewController
    ) -> UIViewController? {
        let index = (viewController as! Page).index
        return index > 0 ? Page(index: index - 1) : nil
    }

    func pageViewController(
        _ pageViewController: UIPageViewController, viewControllerAfter viewController: UIViewController
    ) -> UIViewController? {
        let index = (viewController as! Page).index
        return index < pageColors.count - 1 ? Page(index: index + 1) : nil
    }

    func pageViewController(
        _ pageViewController: UIPageViewController, willTransitionTo pendingViewControllers: [UIViewController]
    ) {
        report("will \((pendingViewControllers.first as! Page).index + 1)")
    }

    func pageViewController(
        _ pageViewController: UIPageViewController, didFinishAnimating finished: Bool,
        previousViewControllers: [UIViewController], transitionCompleted completed: Bool
    ) {
        report("did \((pageViewController.viewControllers!.first as! Page).index + 1) \(completed)")
    }
}

final class Window: UIWindow {
    override func sendEvent(_ event: UIEvent) {
        if let touch = event.allTouches?.first {
            let point = touch.location(in: self)
            let phase =
                switch touch.phase {
                case .began: "began"
                case .moved: "moved"
                case .stationary: "stationary"
                case .ended: "ended"
                case .cancelled: "cancelled"
                default: "other"
                }
            report(String(format: "touch %.4f %@ %.2f %.2f", touch.timestamp, phase, point.x, point.y))
        }
        super.sendEvent(event)
    }
}

@main
final class App: UIResponder, UIApplicationDelegate {
    var window: UIWindow?

    func application(
        _ application: UIApplication, didFinishLaunchingWithOptions options: [UIApplication.LaunchOptionsKey: Any]?
    ) -> Bool {
        let window = Window(frame: UIScreen.main.bounds)
        window.rootViewController = Pages()
        window.makeKeyAndVisible()
        self.window = window
        return true
    }
}
