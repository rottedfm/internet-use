use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "iu", about = r#"
  ,,                                                                                            
  db              mm                                        mm                                  
                  MM                                        MM                                  
`7MM  `7MMpMMMb.mmMMmm .gP"Ya `7Mb,od8 `7MMpMMMb.  .gP"Ya mmMMmm   `7MM  `7MM  ,pP"Ybd  .gP"Ya  
  MM    MM    MM  MM  ,M'   Yb  MM' "'   MM    MM ,M'   Yb  MM       MM    MM  8I   `" ,M'   Yb 
  MM    MM    MM  MM  8M""""""  MM       MM    MM 8M""""""  MM mmmmm MM    MM  `YMMMa. 8M"""""" 
  MM    MM    MM  MM  YM.    ,  MM       MM    MM YM.    ,  MM       MM    MM  L.   I8 YM.    , 
.JMML..JMML  JMML.`Mbmo`Mbmmd'.JMML.   .JMML  JMML.`Mbmmd'  `Mbmo    `Mbod"YML.M9mmmP'  `Mbmmd' 
    "#, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run a job using a URL and prompt
    Open {
        /// The starting URL
        #[arg(short, long)]
        url: String,
    },
}
