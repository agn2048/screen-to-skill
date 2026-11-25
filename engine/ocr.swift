import CoreGraphics
import Foundation
import ImageIO
import Vision

for path in CommandLine.arguments.dropFirst() {
    autoreleasepool {
        var lines: [String] = []
        if let src = CGImageSourceCreateWithURL(URL(fileURLWithPath: path) as CFURL, nil),
           let image = CGImageSourceCreateImageAtIndex(src, 0, nil) {
            let request = VNRecognizeTextRequest()
            request.recognitionLevel = .accurate
            request.usesLanguageCorrection = false
            try? VNImageRequestHandler(cgImage: image, options: [:]).perform([request])
            lines = (request.results ?? []).compactMap { r in
                guard let c = r.topCandidates(1).first, c.confidence >= 0.3 else { return nil }
                return c.string
            }
        }
        if let data = try? JSONSerialization.data(withJSONObject: ["path": path, "lines": lines] as [String: Any]) {
            FileHandle.standardOutput.write(data + Data("\n".utf8))
        }
    }
}
