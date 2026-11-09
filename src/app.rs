use gpui_kit::gpui::{self, Context, IntoElement, PathPromptOptions, Render, Window, div, prelude::*, px, rgb};
use gpui_kit::base::input::{Textarea, TextareaState};
use gpui_kit::component::scroll::ScrollableElement;
use std::{path::PathBuf, thread};

const BG:u32=0x1f1f1f; const SIDEBAR:u32=0x1b1b1b; const PANEL:u32=0x303030;
const BORDER:u32=0x30363b; const TEXT:u32=0xe8e9e9; const MUTED:u32=0x92999f;
const ACCENT:u32=0xff5100; const BLUE:u32=0x3987d7;

#[derive(Debug)]
enum StreamEvent { Chunk(String), Finished(Result<(crate::providers::ollama::ChatUsage,Option<u64>),String>) }

pub struct PatchApp {
    workspace_id:Option<i64>, workspace_path:Option<PathBuf>, workspace_name:String,
    active_chat:Option<i64>, chats:Vec<(i64,String)>,
    messages:Vec<crate::providers::ollama::ChatMessage>, transcript:String,
    models:Vec<String>, selected_model:usize, model_menu_open:bool, build_mode:bool,
    prompt:gpui_kit::gpui::Entity<TextareaState>, busy:bool, status:String,
    usage:crate::usage::UsageLedger, pricing:Option<crate::usage::TokenPricing>,
}

impl PatchApp {
    pub fn new(window:&mut Window,cx:&mut Context<Self>)->Self {
        let prompt=cx.new(|cx| TextareaState::new(window,cx).auto_grow(2,8).placeholder("Ask Patch about your code…"));
        let (tx,rx)=futures::channel::oneshot::channel();
        thread::spawn(move||{let _=tx.send(crate::providers::ollama::OllamaClient::default().list_models());});
        cx.spawn(async move|this,cx|{
            if let Ok(result)=rx.await {
                let _=this.update(cx,|app,cx|{
                    match result {
                        Ok(models) if !models.is_empty()=>{app.models=models.into_iter().map(|m|m.name).collect();app.status=format!("{} local model(s) available",app.models.len());}
                        Ok(_)=>app.status="No local Ollama models found".into(),
                        Err(e)=>app.status=format!("Ollama unavailable: {e}"),
                    }
                    cx.notify();
                });
            }
        }).detach();
        Self {
            workspace_id:None,workspace_path:None,workspace_name:"No project open".into(),
            active_chat:None,chats:Vec::new(),messages:Vec::new(),transcript:String::new(),
            models:Vec::new(),selected_model:0,model_menu_open:false,build_mode:false,
            prompt,busy:false,status:"Discovering local Ollama models…".into(),
            usage:crate::usage::UsageLedger::default(),pricing:None,
        }
    }

    fn open_workspace(&mut self,_:&gpui_kit::gpui::ClickEvent,_:&mut Window,cx:&mut Context<Self>) {
        let receiver=cx.prompt_for_paths(PathPromptOptions{files:false,directories:true,multiple:false,prompt:Some("Open workspace".into())});
        cx.spawn(async move|this,cx|{
            if let Ok(Ok(Some(paths)))=receiver.await {
                if let Some(path)=paths.into_iter().next() {
                    let result=crate::storage::register_workspace(&path);
                    let _=this.update(cx,|app,cx|{
                        match result {
                            Ok(id)=>{
                                app.workspace_id=Some(id);app.workspace_path=Some(path.clone());
                                app.workspace_name=path.file_name().and_then(|n|n.to_str()).unwrap_or("Workspace").to_owned();
                                app.active_chat=None;app.messages.clear();app.transcript.clear();
                                app.usage=crate::usage::UsageLedger::default();
                                app.chats=crate::storage::list_chat_titles(id).unwrap_or_default();
                                app.status=format!("Opened {}",path.display());
                            }
                            Err(e)=>app.status=format!("Could not register workspace: {e}"),
                        }
                        cx.notify();
                    });
                }
            }
        }).detach();
    }

    fn create_chat(&mut self,_:&gpui::ClickEvent,_:&mut Window,cx:&mut Context<Self>) {
        let Some(workspace)=self.workspace_id else {self.status="Open a workspace first".into();cx.notify();return;};
        match crate::storage::create_chat(workspace) {
            Ok(id)=>{self.active_chat=Some(id);self.chats.insert(0,(id,"New chat".into()));self.messages.clear();self.transcript.clear();self.usage=Default::default();self.status="New conversation".into();}
            Err(e)=>self.status=format!("Could not create chat: {e}"),
        }
        cx.notify();
    }

    fn select_chat(&mut self,id:i64,cx:&mut Context<Self>) {
        match crate::storage::list_messages(id) {
            Ok(rows)=>{
                self.active_chat=Some(id);
                self.messages=rows.iter().map(|(role,content)|crate::providers::ollama::ChatMessage{role:role.clone(),content:content.clone()}).collect();
                self.transcript=rows.iter().map(|(role,content)|format!("{}\n{}\n\n",if role=="user"{"You"}else{"Patch"},content)).collect();
                self.usage=Default::default();self.status="Conversation loaded".into();
            }
            Err(e)=>self.status=format!("Could not load conversation: {e}"),
        }
        cx.notify();
    }

    fn toggle_mode(&mut self,_:&gpui::ClickEvent,_:&mut Window,cx:&mut Context<Self>) {
        self.build_mode=!self.build_mode;
        self.status=if self.build_mode{"Build mode · mutations still require permission checks".into()}else{"Plan mode · read-only operations".into()};
        cx.notify();
    }

    fn toggle_models(&mut self,_:&gpui::ClickEvent,_:&mut Window,cx:&mut Context<Self>) {self.model_menu_open=!self.model_menu_open;cx.notify();}

    fn choose_model(&mut self,index:usize,cx:&mut Context<Self>) {
        self.selected_model=index;self.model_menu_open=false;self.usage=Default::default();
        let name=self.models.get(index).cloned().unwrap_or_default();
        self.pricing=crate::storage::get_model_pricing("ollama",&name).ok().flatten().or(Some(crate::usage::TokenPricing{input_usd_per_million:0.0,output_usd_per_million:0.0}));
        self.status=format!("Selected {name}");cx.notify();
    }

    fn send_prompt(&mut self,_:&gpui::ClickEvent,window:&mut Window,cx:&mut Context<Self>) {
        if self.busy{return;}
        let prompt=self.prompt.read(cx).value().trim().to_owned();
        if prompt.is_empty(){return;}
        let Some(chat_id)=self.active_chat else {self.status="Open a workspace and create a chat first".into();cx.notify();return;};
        let Some(model)=self.models.get(self.selected_model).cloned() else {self.status="No Ollama model is selected".into();cx.notify();return;};
        if let Err(e)=crate::storage::save_message(chat_id,"user",&prompt,None,None){self.status=format!("Could not save prompt: {e}");cx.notify();return;}
        self.messages.push(crate::providers::ollama::ChatMessage{role:"user".into(),content:prompt.clone()});
        self.transcript.push_str(&format!("You\n{prompt}\n\nPatch\n"));
        self.busy=true;self.status=format!("Waiting for {model}…");
        self.prompt.update(cx,|state,cx|state.set_value("",window,cx));
        cx.notify();

        let mut request=self.messages.clone();
        request.insert(0,crate::providers::ollama::ChatMessage{
            role:"system".into(),
            content:if self.build_mode {
                "You are Patch, a coding assistant in Build mode. Help implement requested changes. Never claim to have edited files or run commands unless tools performed those actions. Mutations must pass the application's permission policy.".into()
            } else {
                "You are Patch, a coding assistant in Plan mode. Analyse and explain; do not claim to have modified files or run commands. Remain read-only.".into()
            }
        });
        let (tx,rx)=async_channel::unbounded::<StreamEvent>();
        thread::spawn(move||{
            let client=crate::providers::ollama::OllamaClient::default();
            let result=client.chat_stream(&model,&request,|chunk|{let _=tx.send_blocking(StreamEvent::Chunk(chunk));})
                .map(|usage|{let context=client.model_info(&model).ok().and_then(|info|info.context_length);(usage,context)})
                .map_err(|e|e.to_string());
            let _=tx.send_blocking(StreamEvent::Finished(result));
        });
        cx.spawn(async move|this,cx|{
            let mut answer=String::new();
            while let Ok(event)=rx.recv().await {
                let finished=matches!(&event,StreamEvent::Finished(_));
                let _=this.update(cx,|app,cx|{
                    match event {
                        StreamEvent::Chunk(chunk)=>{answer.push_str(&chunk);app.transcript.push_str(&chunk);}
                        StreamEvent::Finished(Ok((stats,limit)))=>{
                            app.transcript.push_str("\n\n");
                            let text=answer.clone();
                            if let Err(e)=crate::storage::save_message(chat_id,"assistant",&text,stats.prompt_tokens,stats.completion_tokens){app.status=format!("Response received; save failed: {e}");}
                            else{app.status="Response complete".into();}
                            app.messages.push(crate::providers::ollama::ChatMessage{role:"assistant".into(),content:text});
                            if let(Some(input),Some(output))=(stats.prompt_tokens,stats.completion_tokens){app.usage.record(input,output,Some(input.saturating_add(output)),limit);}
                            app.busy=false;
                        }
                        StreamEvent::Finished(Err(e))=>{app.transcript.push_str(&format!("\n\n[Ollama error: {e}]\n\n"));app.status=format!("Ollama request failed: {e}");app.busy=false;}
                    }
                    cx.notify();
                });
                if finished{break;}
            }
        }).detach();
    }
}

impl Render for PatchApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mode = if self.build_mode { "BUILD" } else { "PLAN" };
        let mode_colour = if self.build_mode { ACCENT } else { BLUE };
        let model = self.models.get(self.selected_model).cloned().unwrap_or_else(|| "Ollama unavailable".into());
        let counts = self.usage.counts();
        let context = counts.context_fraction().map(|v| format!("CTX {:.0}%", v * 100.0)).unwrap_or_else(|| "CTX —".into());
        let cost = match self.usage.cost(self.pricing) {
            crate::usage::CostDisplay::Known(v) => format!("USD {:.4}", v),
            crate::usage::CostDisplay::Unknown => "Cost —".into(),
        };
        let transcript = self.transcript.clone();
        let chats = self.chats.clone();
        let models = self.models.clone();
        let menu = self.model_menu_open;
        let active = self.active_chat;
        let workspace = self.workspace_name.clone();
        let status = self.status.clone();
        let busy = self.busy;
        let build = self.build_mode;

        let workspace_hint = if let Some(path) = &self.workspace_path {
            div().text_size(px(10.)).text_color(rgb(MUTED)).child(path.display().to_string())
        } else {
            div().text_size(px(10.)).text_color(rgb(MUTED)).child("Choose a folder to begin")
        };

        let chat_list = if chats.is_empty() {
            div().text_size(px(12.)).text_color(rgb(MUTED)).child("Chats for this workspace will appear here.")
        } else {
            div().flex().flex_col().gap_1().children(chats.iter().map(|(id, title)| {
                let id = *id;
                let selected = active == Some(id);
                div().id(("chat-row", id as u64)).w_full().px_2().py_2().rounded_md()
                    .bg(rgb(if selected { PANEL } else { SIDEBAR }))
                    .text_size(px(12.)).cursor_pointer().child(title.clone())
                    .on_click(cx.listener(move |this, _, _, cx| this.select_chat(id, cx)))
            }))
        };

        let sidebar = div().w(px(260.)).h_full().flex().flex_col().gap_4().p_4()
            .bg(rgb(SIDEBAR)).border_r_1().border_color(rgb(BORDER))
            .child(
                div().flex().items_center()
                    .child(div().text_size(px(25.)).font_weight(gpui_kit::gpui::FontWeight::BLACK)
                        .text_color(rgb(ACCENT)).child("P A T C H"))
            )
            .child(
                div().id("new-chat").w_full().px_3().py_2().rounded_md()
                    .bg(rgb(ACCENT)).text_color(rgb(0xffffff)).cursor_pointer()
                    .child("+   New chat").on_click(cx.listener(Self::create_chat))
            )
            .child(
                div().flex().flex_col().gap_2()
                    .child(div().text_size(px(10.)).font_weight(gpui::FontWeight::BOLD).text_color(rgb(MUTED)).child("WORKSPACE"))
                    .child(
                        div().id("open-workspace").w_full().px_3().py_3().rounded_md()
                            .bg(rgb(PANEL)).border_1().border_color(rgb(BORDER)).cursor_pointer()
                            .child(format!("▣  {workspace}")).on_click(cx.listener(Self::open_workspace))
                    )
                    .child(workspace_hint)
            )
            .child(
                div().flex().flex_col().gap_2().flex_1()
                    .child(div().text_size(px(10.)).font_weight(gpui::FontWeight::BOLD).text_color(rgb(MUTED)).child("CHATS"))
                    .child(chat_list)
            )
            .child(div().flex_1())
            .child(div().text_size(px(11.)).text_color(rgb(MUTED)).child("PATCH  ·  GPUI"));

        let header = div().h(px(50.)).w_full().px_4().flex().items_center().justify_between()
            .border_b_1().border_color(rgb(BORDER))
            .child(
                div().id("model-selector").px_3().py_2().rounded_md().bg(rgb(PANEL))
                    .border_1().border_color(rgb(BORDER)).text_size(px(12.)).cursor_pointer()
                    .child(format!("{model}  ▾")).on_click(cx.listener(Self::toggle_models))
            )
            .child(div().text_size(px(13.)).font_weight(gpui::FontWeight::BOLD).child(workspace.clone()))
            .child(
                div().flex().items_center().gap_2()
                    .child(div().px_2().py_2().text_size(px(10.)).text_color(rgb(MUTED)).child(context))
                    .child(div().px_2().py_2().text_size(px(10.)).text_color(rgb(MUTED)).child(cost))
            );

        let model_menu = if menu {
            div().w_full().px_6().py_2().bg(rgb(SIDEBAR)).flex().flex_col().gap_1()
                .children(models.iter().enumerate().map(|(index, name)| {
                    div().id(("model-choice", index)).px_3().py_2().rounded_md()
                        .bg(rgb(if index == self.selected_model { PANEL } else { SIDEBAR }))
                        .text_size(px(12.)).cursor_pointer().child(name.clone())
                        .on_click(cx.listener(move |this, _, _, cx| this.choose_model(index, cx)))
                }))
        } else {
            div().h(px(0.))
        };

        let transcript_view = if transcript.is_empty() {
            div().flex_1().flex().flex_col().items_center().justify_center()
                .child(div().text_size(px(42.)).font_weight(gpui::FontWeight::BLACK)
                    .text_color(rgb(ACCENT)).child("P A T C H"))
        } else {
            div().w_full().max_w(px(900.)).mx_auto().text_size(px(13.)).child(transcript)
        };

        let conversation = div().flex_1().w_full().px_8().py_5().flex().flex_col()
            .gap_3().overflow_y_scrollbar().child(transcript_view);

        let composer_box = div().w_full().p_3().rounded_lg()
            .bg(rgb(PANEL)).border_1()
            .border_color(rgb(if build { ACCENT } else { BLUE }))
            .child(Textarea::new(&self.prompt));

        let composer_footer = div().w_full().flex()
            .items_center().justify_between()
            .child(
                div().flex().items_center().gap_2()
                    .child(
                        div().id("mode-toggle").px_3().py_2().rounded_md().bg(rgb(mode_colour))
                            .text_color(rgb(0xffffff)).text_size(px(11.)).cursor_pointer()
                            .child(mode).on_click(cx.listener(Self::toggle_mode))
                    )
                    .child(div().text_size(px(10.)).text_color(rgb(MUTED)).child("Tab to switch modes"))
            )
            .child(
                div().flex().items_center().gap_3()
                    .child(div().text_size(px(10.)).text_color(rgb(MUTED)).child(status))
                    .child(
                        div().id("send-prompt").px_4().py_2().rounded_md()
                            .bg(rgb(if busy { BORDER } else { ACCENT }))
                            .text_color(rgb(0xffffff)).text_size(px(12.)).cursor_pointer()
                            .child(if busy { "Working…" } else { "Send  ↗" })
                            .on_click(cx.listener(Self::send_prompt))
                    )
            );

        let composer = div().w_full().px_8().pb_4().flex().flex_col().gap_2()
            .child(composer_box).child(composer_footer);

        let main = div().flex_1().h_full().flex().flex_col()
            .child(header).child(model_menu).child(conversation).child(composer);

        div().size_full().flex().bg(rgb(BG)).text_color(rgb(TEXT))
            .child(sidebar).child(main)
    }
}
