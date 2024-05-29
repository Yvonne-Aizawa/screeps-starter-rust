use screeps::RectStyle;

pub trait VisualExtend {
    fn draw_progress_bar(
        self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        procent: f32,
        front_style: Option<RectStyle>,
        back_style: Option<RectStyle>,
        label: Option<String>,
    ) -> Self;
}
