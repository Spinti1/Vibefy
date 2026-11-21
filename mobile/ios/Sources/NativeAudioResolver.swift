import Foundation
import YouTubeKit

/// Only public metadata is resolved here. Account credentials and media files are never stored.
actor NativeAudioResolver {
 private var cache:[String:(URL,Date)]=[:]
 func resolve(_ video:String) async throws -> URL {
  guard video.range(of:"^[A-Za-z0-9_-]{11}$",options:.regularExpression) != nil else{throw URLError(.badURL)}
  if let held=cache[video],held.1>Date(){return held.0}
  let streams=try await YouTube(videoID:video,methods:[.local]).streams
  try Task.checkCancellation()
  guard let stream=streams.filterAudioOnly().filter({$0.fileExtension == .m4a && $0.isNativelyPlayable}).highestAudioBitrateStream() else{throw URLError(.cannotDecodeContentData)}
  let url=stream.url
  guard url.scheme=="https",let host=url.host,host.hasSuffix(".googlevideo.com"),url.user==nil,url.password==nil,(url.port==nil || url.port==443) else{throw URLError(.badURL)}
  try await stream.waitUntilAvailable();try Task.checkCancellation()
  cache=cache.filter{$0.value.1>Date()};if cache.count>=8,let key=cache.keys.first{cache.removeValue(forKey:key)}
  cache[video]=(url,Date().addingTimeInterval(240));return url
 }
}
