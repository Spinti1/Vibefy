import SwiftUI

import AVFoundation

import MediaPlayer



struct Song: Codable, Identifiable, Equatable {

 var id:String;var title:String;var artist:String;var duration:Double?;var image:String?

 var audioURL:URL? {

  if id.hasPrefix("jv:"),UUID(uuidString:String(id.dropFirst(3))) != nil {return URL(string:"https://api.juicevault.xyz/music/stream/"+id.dropFirst(3))}

  if id.range(of:"^[A-Za-z0-9_-]{11}$",options:.regularExpression) != nil {return URL(string:"https://vibefy-neon.vercel.app/api/audio/"+id)}

  return nil

 }

}

struct NativePlaylist:Codable,Identifiable {var id=UUID();var title:String;var tracks:[Song]=[]}

struct SearchResponse:Decodable {var tracks:[Song]}

struct PlaylistResponse:Decodable {var title:String;var tracks:[Song]}



@MainActor final class Library:ObservableObject {

 @Published var playlists:[NativePlaylist]=[]

 @Published var error=""

 private let file:URL

 private let writer=DispatchQueue(label:"online.usevibefy.library")

 init(){

  file=FileManager.default.urls(for:.applicationSupportDirectory,in:.userDomainMask)[0].appendingPathComponent("Vibefy/library.json")

  let file=file

  Task {

   do {let data=try await Task.detached{try Data(contentsOf:file)}.value;playlists=try JSONDecoder().decode([NativePlaylist].self,from:data)}

   catch {if FileManager.default.fileExists(atPath:file.path){self.error="Library could not be read. The saved file has been retained."}}

  }

 }

 func save(){

  do {let data=try JSONEncoder().encode(playlists);let file=file;writer.async {

   do {try FileManager.default.createDirectory(at:file.deletingLastPathComponent(),withIntermediateDirectories:true);try data.write(to:file,options:[.atomic,.completeFileProtectionUntilFirstUserAuthentication])}

   catch {Task{@MainActor in self.error="Playlist changes could not be saved."}}

  }}catch {self.error="Playlist data could not be encoded."}

 }

 func create(_ name:String){let name=name.trimmingCharacters(in:.whitespacesAndNewlines);guard !name.isEmpty else{return};playlists.append(NativePlaylist(title:name));save()}

 func add(_ song:Song,to id:UUID){guard let i=playlists.firstIndex(where:{$0.id==id}),!playlists[i].tracks.contains(where:{$0.id==song.id})else{return};playlists[i].tracks.append(song);save()}

 func fetch<T:Decodable>(_ path:String,as type:T.Type) async throws->T {

  guard let url=URL(string:"https://vibefy-neon.vercel.app"+path)else{throw URLError(.badURL)}

  let (data,response)=try await URLSession.shared.data(from:url)

  guard let http=response as? HTTPURLResponse,http.statusCode==200,data.count<2_000_000 else{throw URLError(.badServerResponse)}

  return try JSONDecoder().decode(type,from:data)

 }

}

@MainActor final class Player:ObservableObject {

 @Published var queue:[Song]=[];@Published var index=0;@Published var playing=false;@Published var position=0.0;@Published var duration=1.0;@Published var error="";@Published var shuffled=false;@Published var repeating=false

 private let engine=AVPlayer();private var tick:Any?;private var end:NSObjectProtocol?;private var status:NSKeyValueObservation?;private var controls:[(MPRemoteCommand,Any)]=[]

 var song:Song?{queue.indices.contains(index) ? queue[index]:nil}

 init(){

  do {try AVAudioSession.sharedInstance().setCategory(.playback,mode:.default)}catch {self.error="Audio session could not be configured."}

  tick=engine.addPeriodicTimeObserver(forInterval:CMTime(seconds:0.5,preferredTimescale:600),queue:.main){[weak self] time in

   Task{@MainActor in guard let self else{return};let seconds=time.seconds;self.position=seconds.isFinite ? seconds:0;let total=self.engine.currentItem?.duration.seconds ?? 0;self.duration=total.isFinite && total>0 ? total:1;self.playing=self.engine.timeControlStatus == .playing;self.updateNowPlaying()}

  }

  let center=MPRemoteCommandCenter.shared()

  controls.append((center.playCommand,center.playCommand.addTarget{[weak self] _ in Task{@MainActor in self?.resume()};return .success}))

  controls.append((center.pauseCommand,center.pauseCommand.addTarget{[weak self] _ in Task{@MainActor in self?.pause()};return .success}))

  controls.append((center.nextTrackCommand,center.nextTrackCommand.addTarget{[weak self] _ in Task{@MainActor in self?.next()};return .success}))

  controls.append((center.previousTrackCommand,center.previousTrackCommand.addTarget{[weak self] _ in Task{@MainActor in self?.previous()};return .success}))

  controls.append((center.changePlaybackPositionCommand,center.changePlaybackPositionCommand.addTarget{[weak self] event in guard let event=event as? MPChangePlaybackPositionCommandEvent else{return .commandFailed};Task{@MainActor in self?.seek(event.positionTime)};return .success}))

 }

 func start(_ songs:[Song],at selected:Int){guard songs.indices.contains(selected)else{return};queue=songs;index=selected;load()}

 func load(){

  guard let song,let url=song.audioURL else{error="This source is not supported yet.";return}

  error="";engine.pause();playing=false;position=0

  if let end {NotificationCenter.default.removeObserver(end)}

  let item=AVPlayerItem(url:url);engine.replaceCurrentItem(with:item)

  status=item.observe(\.status,options:[.new]){[weak self] item,_ in if item.status == .failed {Task{@MainActor in self?.error="This source could not be played. YouTube engine port is still in development.";self?.playing=false}}}

  end=NotificationCenter.default.addObserver(forName:.AVPlayerItemDidPlayToEndTime,object:item,queue:.main){[weak self] _ in Task{@MainActor in if self?.repeating == true {self?.seek(0);self?.resume()}else{self?.next()}}}

  resume()

 }

 func resume(){do {try AVAudioSession.sharedInstance().setActive(true);engine.play()}catch {self.error="Audio playback could not start."}}

 func pause(){engine.pause();playing=false;updateNowPlaying()}

 func toggle(){playing ? pause():resume()}

 func next(){guard !queue.isEmpty else{return};if shuffled && queue.count>1 {let options=queue.indices.filter{$0 != index};index=options.randomElement()!}else{guard index+1<queue.count else{pause();return};index+=1};load()}

 func previous(){if position>3 {seek(0)}else if index>0 {index-=1;load()}}

 func seek(_ value:Double){guard value.isFinite else{return};engine.seek(to:CMTime(seconds:max(0,min(value,duration)),preferredTimescale:600));position=value}

 func updateNowPlaying(){guard let song else{return};MPNowPlayingInfoCenter.default().nowPlayingInfo=[MPMediaItemPropertyTitle:song.title,MPMediaItemPropertyArtist:song.artist,MPMediaItemPropertyPlaybackDuration:duration,MPNowPlayingInfoPropertyElapsedPlaybackTime:position,MPNowPlayingInfoPropertyPlaybackRate:playing ? 1:0]}

}

@main struct VibefyApp:App {

 @StateObject private var library=Library();@StateObject private var player=Player()

 var body:some Scene {WindowGroup {RootView().environmentObject(library).environmentObject(player).preferredColorScheme(.dark).tint(Color(red:0,green:0.83,blue:0.75))}}

}

struct RootView:View {

 @EnvironmentObject var library:Library;@EnvironmentObject var player:Player

 @State private var create=false;@State private var name="";@State private var importing=false;@State private var link="";@State private var spotify=false;@State private var busy=false

 var body:some View {

  VStack(spacing:0){

   TabView {

    NavigationView {List {

     Section {Button("Create playlist",systemImage:"plus"){create=true};Button("Import YouTube playlist",systemImage:"arrow.down.circle"){spotify=false;importing=true};Button("Import public Spotify playlist",systemImage:"arrow.down.circle"){spotify=true;importing=true}}

     Section("Your playlists") {ForEach(library.playlists){list in NavigationLink {SongsView(songs:list.tracks,title:list.title)}label:{HStack {Image(systemName:"music.note.list").font(.title2).foregroundStyle(.teal);VStack(alignment:.leading){Text(list.title).font(.headline);Text("\(list.tracks.count) songs").font(.caption).foregroundStyle(.secondary)}}}}}

     if busy {ProgressView("Importing playlist…")}

     if !library.error.isEmpty {Text(library.error).foregroundStyle(.red)}

    }.navigationTitle("Vibefy")}.tabItem{Label("Home",systemImage:"house.fill")}

    SearchView().tabItem{Label("Search",systemImage:"magnifyingglass")}

    NavigationView{SongsView(songs:player.queue,title:"Queue")}.tabItem{Label("Queue",systemImage:"list.bullet")}

    NavigationView {Form {Section("Playback"){Toggle("Shuffle",isOn:$player.shuffled);Toggle("Repeat song",isOn:$player.repeating)};Section("Development status"){Text("Native SwiftUI interface and AVPlayer. YouTube source resolution, account connections, Jam and mods are still being ported.")};Section("Storage"){Text("Playlist metadata is saved on this device. Music files are not included. Uninstalling can remove local playlists.")}}.navigationTitle("Settings")}.tabItem{Label("Settings",systemImage:"gearshape")}

   }

   if let song=player.song {VStack(spacing:8){HStack{VStack(alignment:.leading){Text(song.title).font(.headline).lineLimit(1);Text(song.artist).font(.caption).foregroundStyle(.secondary)};Spacer();Button(action:player.previous){Image(systemName:"backward.end.fill")};Button(action:player.toggle){Image(systemName:player.playing ? "pause.circle.fill":"play.circle.fill").font(.largeTitle)};Button(action:player.next){Image(systemName:"forward.end.fill")}};Slider(value:Binding(get:{min(player.position,player.duration)},set:player.seek),in:0...max(1,player.duration))}.padding().background(Color(white:0.1))}

   if !player.error.isEmpty{Text(player.error).font(.caption).foregroundStyle(.red).padding(8)}

  }

  .alert("Create playlist",isPresented:$create){TextField("Playlist name",text:$name);Button("Create"){library.create(name);name=""};Button("Cancel",role:.cancel){}}

  .alert(spotify ? "Public Spotify playlist":"YouTube playlist",isPresented:$importing){TextField("Playlist link",text:$link);Button("Import"){importPlaylist()};Button("Cancel",role:.cancel){}}message:{Text("Paste a public playlist link. Only playlist metadata is saved.")}

 }

 private func importPlaylist(){

  let components=URLComponents(string:link);let identity=spotify ? components?.url?.lastPathComponent:components?.queryItems?.first(where:{$0.name=="list"})?.value

  guard let identity,identity.range(of:spotify ? "^[A-Za-z0-9]{22}$":"^[A-Za-z0-9_-]{10,200}$",options:.regularExpression) != nil else{library.error="Enter a valid public playlist link.";return}

  busy=true;Task {defer{busy=false};do {let response=try await library.fetch((spotify ? "/api/import/spotify?id=":"/api/playlist?id=")+identity,as:PlaylistResponse.self);library.playlists.append(NativePlaylist(title:response.title,tracks:response.tracks));library.save()}catch {library.error="Playlist import failed. Try again later."}}

 }

}

struct SongsView:View {

 @EnvironmentObject var player:Player;@EnvironmentObject var library:Library

 var songs:[Song];var title:String

 var body:some View {List {ForEach(Array(songs.enumerated()),id:\.offset){index,song in

  Button {player.start(songs,at:index)}label:{HStack {AsyncImage(url:URL(string:song.image ?? "")){image in image.resizable().scaledToFill()}placeholder:{Image(systemName:"music.note")}.frame(width:48,height:48).clipped().cornerRadius(6);VStack(alignment:.leading){Text(song.title).foregroundStyle(.primary).lineLimit(1);Text(song.artist).font(.caption).foregroundStyle(.secondary).lineLimit(1)}}}

  .contextMenu{Menu("Add to playlist"){ForEach(library.playlists){list in Button(list.title){library.add(song,to:list.id)}}}}

 }}.navigationTitle(title)}

}

struct SearchView:View {

 @EnvironmentObject var library:Library

 @State private var query="";@State private var songs:[Song]=[];@State private var searching=false;@State private var generation=UUID();@State private var message=""

 var body:some View {NavigationView {VStack {if searching {ProgressView()};if !message.isEmpty {Text(message).foregroundStyle(.secondary)};SongsView(songs:songs,title:"Search")}.searchable(text:$query,prompt:"Songs and artists").onSubmit(of:.search){search()}}}

 private func search(){let query=query.trimmingCharacters(in:.whitespacesAndNewlines);guard !query.isEmpty else{return};let ticket=UUID();generation=ticket;searching=true;message="";Task {do{let value=try await library.fetch("/api/search?q="+(query.addingPercentEncoding(withAllowedCharacters:.urlQueryAllowed) ?? ""),as:SearchResponse.self);if generation==ticket {songs=value.tracks;searching=false;if songs.isEmpty {message="No matching songs found."}}}catch {if generation==ticket {message="Search unavailable. Try again later.";searching=false}}}}

}

